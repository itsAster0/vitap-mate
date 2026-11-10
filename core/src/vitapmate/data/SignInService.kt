package vitapmate.data

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.Looper

/**
 * Keeps the app running while the student signs in to Google in the browser, so Android doesn't
 * freeze it or cut its network: the loopback listener answers at once and the token exchange
 * finishes before they come back. A `shortService` on Android 14+ (no extra permission), a plain
 * foreground service before; it stops itself after [LIMIT_MS] either way.
 */
class SignInService : Service() {
    private val handler = Handler(Looper.getMainLooper())
    private val end = Runnable { stopSelf() }

    override fun onBind(intent: Intent?) = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val manager = getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel(CHANNEL, "Sign-in", NotificationManager.IMPORTANCE_LOW))
        val icon = resources.getIdentifier("ic_notification", "drawable", packageName).takeIf { it != 0 } ?: applicationInfo.icon
        val back = packageManager.getLaunchIntentForPackage(packageName)?.let {
            PendingIntent.getActivity(this, 0, it, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        }
        val notification = Notification.Builder(this, CHANNEL)
            .setSmallIcon(icon)
            .setContentTitle("Connecting Gmail…")
            .setContentText("Finish in the browser")
            .setOngoing(true)
            .setContentIntent(back)
            .build()
        if (Build.VERSION.SDK_INT >= 34) {
            startForeground(ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_SHORT_SERVICE)
        } else {
            startForeground(ID, notification)
        }
        handler.removeCallbacks(end)
        handler.postDelayed(end, LIMIT_MS)
        return START_NOT_STICKY
    }

    /** Android 14+ ends a short service after about 3 minutes; stop before it counts as an ANR. */
    override fun onTimeout(startId: Int) = stopSelf()

    override fun onDestroy() {
        handler.removeCallbacks(end)
        super.onDestroy()
    }

    companion object {
        private const val CHANNEL = "sign_in"
        private const val ID = 0x5167
        private const val LIMIT_MS = 3 * 60_000L

        /** From the foreground only: Android refuses a foreground service started in the background. */
        fun start(context: Context) {
            runCatching { context.startForegroundService(Intent(context, SignInService::class.java)) }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, SignInService::class.java))
        }
    }
}
