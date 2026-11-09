package vitapmate

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.work.Constraints
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.ExistingWorkPolicy
import androidx.work.NetworkType
import androidx.work.OneTimeWorkRequestBuilder
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Change
import vitapmate.core.ChangeKind
import vitapmate.core.Core
import vitapmate.data.AppCore
import vitapmate.data.Settings
import vitapmate.data.StoreCap
import vitapmate.data.VtopSync
import vitapmate.data.coreFlow
import vitapmate.features.login.MailForwarding
import vitapmate.features.login.OtpAutofill
import vitapmate.observability.ActivityHistory
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * Settings → Background sync: a WorkManager job every [Settings.syncHours] (on a network) that
 * fetches VTOP's pages ([VtopSync]), so Rust records what changed, then notifies about the
 * changes of the kinds switched on. Offered only while OTP auto-fill works; a run whose OTP can't
 * be filled stops quietly and marks sync paused ([Settings.syncPaused]) until a run goes through.
 * Flow `sync.background`, log src `sync`.
 */
object BackgroundSync {
    private const val WORK = "vtop_sync"
    const val CHANNEL = "changes"

    /** Whether it can be turned on: OTP auto-fill is set up and working. */
    fun available(): Boolean = OtpAutofill.inUse() != null

    /** Schedules or cancels the job to match the settings; call after any of them changes. */
    fun apply(context: Context) {
        val work = WorkManager.getInstance(context)
        if (!Settings.backgroundSync) {
            work.cancelUniqueWork(WORK)
            return
        }
        val hours = Settings.syncHours.toLong()
        val request = PeriodicWorkRequestBuilder<SyncWorker>(hours, TimeUnit.HOURS)
            .setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build())
            .setInitialDelay(hours, TimeUnit.HOURS)
            .build()
        work.enqueueUniquePeriodicWork(WORK, ExistingPeriodicWorkPolicy.UPDATE, request)
    }

    /** One run now, as a scheduled one would (Developer → Run background sync now). */
    fun runNow(context: Context) {
        WorkManager.getInstance(context).enqueueUniqueWork(
            "${WORK}_now",
            ExistingWorkPolicy.REPLACE,
            OneTimeWorkRequestBuilder<SyncWorker>().setConstraints(Constraints.Builder().setRequiredNetworkType(NetworkType.CONNECTED).build()).build(),
        )
    }

    /**
     * Debuggable builds only: `adb shell am broadcast -a vitapmate.SYNC_NOW -p com.vitap_pal.app`
     * starts [runNow], for testing without tapping through Developer; with `--ez simulate true`
     * it first rolls the saved pages back (Developer → Simulate VTOP changes).
     */
    fun registerDebugTrigger(context: Context) {
        val app = context.applicationContext
        if (app.applicationInfo.flags and android.content.pm.ApplicationInfo.FLAG_DEBUGGABLE == 0) return
        val receiver = object : android.content.BroadcastReceiver() {
            override fun onReceive(c: Context, intent: Intent) {
                if (!intent.getBooleanExtra("simulate", false)) return runNow(app)
                kotlinx.coroutines.MainScope().launch {
                    coreFlow("dev.simulate_changes", Core::devSimulateChanges)
                    runNow(app)
                }
            }
        }
        val filter = android.content.IntentFilter("vitapmate.SYNC_NOW")
        if (Build.VERSION.SDK_INT >= 33) app.registerReceiver(receiver, filter, Context.RECEIVER_EXPORTED) else app.registerReceiver(receiver, filter)
    }

    /** A sample change through [notify] (Developer → Show a test notification). */
    fun notifyTest(context: Context) = notify(
        context,
        listOf(Change("test", ChangeKind.ATTENDANCE, System.currentTimeMillis(), "Attendance posted", "Sample course (Theory): 1 class present, now 92%", null, "")),
    )

    /** Posts one notification per kind for [changes] (newest first); tapping opens the inbox. */
    internal fun notify(context: Context, changes: List<Change>) {
        if (changes.isEmpty()) return
        if (Build.VERSION.SDK_INT >= 33 && ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) return
        val manager = context.getSystemService(NotificationManager::class.java)
        if (Build.VERSION.SDK_INT >= 26) {
            manager.createNotificationChannel(NotificationChannel(CHANNEL, "VTOP changes", NotificationManager.IMPORTANCE_DEFAULT).apply { description = "What changed on VTOP since you last looked" })
        }
        val open = PendingIntent.getActivity(
            context, 0,
            Intent(context, MainActivity::class.java).putExtra(MainActivity.OPEN, MainActivity.OPEN_INBOX).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        for ((kind, list) in changes.groupBy { it.kind }) {
            val title = if (list.size == 1) list[0].title else "${list[0].title} and ${list.size - 1} more"
            // One change: its text in full; several: a line each.
            val style = if (list.size == 1) NotificationCompat.BigTextStyle().bigText(list[0].body)
            else NotificationCompat.InboxStyle().also { s -> list.take(6).forEach { s.addLine("${it.title}: ${it.body}") } }
            val n = NotificationCompat.Builder(context, CHANNEL)
                .setSmallIcon(R.drawable.ic_notification)
                .setContentTitle(title)
                .setContentText(list[0].body)
                .setStyle(style)
                .setContentIntent(open)
                .setAutoCancel(true)
                .build()
            NotificationManagerCompat.from(context).notify(kind.ordinal + 100, n)
        }
    }
}

class SyncWorker(context: Context, params: WorkerParameters) : CoroutineWorker(context, params) {
    override suspend fun doWork(): Result {
        val app = applicationContext
        Settings.load(app)
        AppCore.start(app)
        if (!Settings.backgroundSync) return Result.success()
        val prefs = app.getSharedPreferences("sync", Context.MODE_PRIVATE)
        flow("sync.background") { f ->
            MailForwarding.load("sync")
            if (!BackgroundSync.available()) {
                Settings.syncPaused = if (OtpAutofill.selected().name == "Gmail") "Gmail needs reconnecting" else "OTP auto-fill isn't set up"
                Log.info("sync", "skipped: ${Settings.syncPaused}", f)
                return@flow
            }
            val result = VtopSync.run("sync.background.pages", "Background sync", everything = false, pageTimeoutMs = PAGE_TIMEOUT_MS) { _, id ->
                val why = OtpAutofill.solve(id)
                if (why != null) Settings.syncPaused = why
                why == null
            }
            if (result.outcome == VtopSync.Outcome.Done) Settings.syncPaused = null
            // New changes since the last notification, of the kinds switched on.
            val since = prefs.getLong("notified_ms", System.currentTimeMillis() - 86_400_000L)
            val fresh = withContext(Dispatchers.IO) { AppCore.await().changes(f) }
                .filter { it.atMs > since && Settings.notifies(it.kind.name.lowercase()) }
            BackgroundSync.notify(app, fresh)
            prefs.edit().putLong("notified_ms", System.currentTimeMillis()).apply()
            Log.info("sync", "background done", f, mapOf("outcome" to result.outcome.name.lowercase(), "fetched" to "${result.fetched}", "notified" to "${fresh.size}"))
            withContext(Dispatchers.IO) { ActivityHistory.save() }
            StoreCap.enforce(app, f)
        }
        return Result.success()
    }

    private companion object {
        /** Each page's limit: short, so a slow VTOP ends the run instead of draining battery. */
        const val PAGE_TIMEOUT_MS = 20_000L
    }
}
