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
import com.google.firebase.messaging.FirebaseMessaging
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.awaitCancellation
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import vitapmate.core.LiveTopic
import vitapmate.data.AccountState
import vitapmate.data.AppCore
import vitapmate.data.AppScope
import vitapmate.data.Foreground
import vitapmate.data.Live
import vitapmate.data.Settings
import vitapmate.data.attempt
import vitapmate.data.coreFlow
import vitapmate.data.Tunable
import vitapmate.data.problemOf
import vitapmate.data.toasts
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.showToast

/**
 * FCM: a new token goes to the server with this phone's device; a `vtop_cookie_request` is
 * coordinated through [HandoverCoordinator] (notifying off screen, answering on screen alongside
 * [Push.follow]); a `credentials_rejected` flags the account and notifies ([Push.rejected]);
 * other messages are only logged. Tokens and message contents are never logged.
 * Log src `push`.
 */
class PushService : FirebaseMessagingService() {
    // Deprecated in firebase-messaging 26 for register()/onRegistered, but that doesn't hand
    // back the token on later starts; these still work.
    @Deprecated("FCM 26")
    @Suppress("DEPRECATION")
    override fun onNewToken(token: String) = Push.send(token)

    override fun onMessageReceived(message: RemoteMessage) {
        val data = message.data
        val kind = data["kind"] ?: "-"
        if (kind == "credentials_rejected") {
            AppScope.launch { flow("push.credentials_rejected") { f -> Push.rejected(applicationContext, f) } }
            return
        }
        if (kind != "vtop_cookie_request") {
            AppScope.launch { flow("push.message") { f -> Log.info("push", "message", f, mapOf("kind" to kind)) } }
            return
        }
        val requestId = data["requestId"]?.takeIf { it.isNotBlank() } ?: return
        val expiresAtMs = HandoverCoordinator.parsePushExpiresAtMs(data)
        if (Foreground.shown) {
            // Don't drop FCM while on screen: Push.follow may be in backoff or the app may
            // background before the live query delivers it.
            AppScope.launch { Push.handleRequest(applicationContext, requestId, expiresAtMs) }
            return
        }
        // FCM keeps the process for this call only, so it runs here (a binder thread).
        Settings.load(applicationContext)
        AppCore.start(applicationContext)
        runBlocking { Push.handleRequest(applicationContext, requestId, expiresAtMs) }
    }
}

object Push {
    /** Sends the current token once the app has started (FCM may not call [PushService.onNewToken] again). */
    @Suppress("DEPRECATION")
    fun start() {
        runCatching { FirebaseMessaging.getInstance().token.addOnSuccessListener { send(it) } }
            .onFailure { Log.warn("push", "no firebase: ${it::class.simpleName}") }
    }

    /**
     * Handles a key's session request (`vtop_cookie_request`) from FCM, `sessions:pending`, or a
     * notification tap through [coordinator]: posts a per-request notification when backgrounded,
     * drops expired requests cleanly, and answers on screen within [Tunable.SessionAnswer] plus
     * [Tunable.CookieAsk]. Flow `push.session_request`; no cookies in the log.
     */
    suspend fun handleRequest(
        context: Context,
        requestId: String,
        expiresAtMs: Long? = null,
        tapped: Boolean = false,
    ) {
        val app = context.applicationContext
        when (val action = coordinator.onRequest(requestId, expiresAtMs, foreground = Foreground.shown, tapped = tapped)) {
            is HandoverAction.Notify -> flow("push.session_request") { f ->
                asked(app, requestId, action.notificationId, action.expiresAtMs)
                Log.info("push", "session request asked by notification", f)
            }
            is HandoverAction.Answer -> {
                cancelNotification(app, action.notificationId)
                AppScope.launch { answerAttempt(app, requestId, action.attemptId) }
            }
            is HandoverAction.Expired -> flow("push.session_request") { f ->
                cancelNotification(app, action.notificationId)
                Log.info("push", "session request expired", f)
            }
            HandoverAction.Ignore -> if (tapped) {
                cancelNotification(app, HandoverCoordinator.notificationId(requestId))
                flow("push.session_request") { f ->
                    Log.info("push", "session request already handled", f)
                }
            }
        }
    }

    /** Promotes any background-notified session requests when the app enters the foreground. */
    fun onForeground(context: Context) {
        val app = context.applicationContext
        val promotion = coordinator.promoteNotified()
        for (nid in promotion.expiredNotificationIds) cancelNotification(app, nid)
        for (request in promotion.toAnswer) {
            cancelNotification(app, request.notificationId)
            AppScope.launch { answerAttempt(app, request.requestId, request.attemptId) }
        }
    }

    private suspend fun answerAttempt(context: Context, requestId: String, attemptId: Long) =
        flow("push.session_request") { f ->
            val limit = (Settings.tunable(Tunable.SessionAnswer) + Settings.tunable(Tunable.CookieAsk)) * 1000L
            val result = coordinator.executeAttempt(
                requestId = requestId,
                attemptId = attemptId,
                timeoutMs = limit,
                timeoutError = TIMEOUT_ERROR,
            ) { error ->
                AppCore.await().answerSessionRequest(f, requestId, error)
            }
            cancelNotification(context, HandoverCoordinator.notificationId(requestId))
            Log.info("push", "session request", f, mapOf("answered" to "${result.isSuccess}", "error" to (result.exceptionOrNull()?.let { it::class.simpleName } ?: "-")))
            // On screen, say so: the app waiting on it only times out.
            result.exceptionOrNull()?.let { e ->
                val problem = problemOf(e, f)
                if (Foreground.shown && problem.toasts) showToast("Couldn't answer a connected app", problem.message)
            }
        }

    private const val ASKED = "connected_apps"
    private const val REJECTED = "account"
    private const val ASKED_TIMEOUT_MS = 60_000L
    private const val TIMEOUT_ERROR = "The phone took too long to sign in to VTOP."
    private const val FOLLOW_RETRY_MS = 30_000L
    private const val FOLLOW_RETRY_MAX_MS = 5 * 60_000L

    private val coordinator = HandoverCoordinator()

    /**
     * Answers each session request waiting on this phone as the server lists it
     * (`sessions:pending`, live) while the app runs; FCM only wakes an app that isn't. Flow
     * `push.pending` per change. Without a server identity (none until the student shares their
     * registration number and the phone proves who they are) the watch can't follow, so it waits
     * for Rust to say one came. A watch that ends anyway is tried again with back-off. Flow
     * `push.follow` while on screen.
     */
    suspend fun follow(context: Context): Nothing = Foreground.shownFlow.collectLatest { shown ->
        // Off screen, FCM brings requests and the socket may close.
        if (!shown) return@collectLatest
        onForeground(context)
        flow("push.follow") { f ->
            var wait = FOLLOW_RETRY_MS
            while (true) {
                // No core, no identity ever: nothing to follow until the app leaves the screen.
                if (attempt { AppCore.await().waitServerIdentity(f) }.isFailure) awaitCancellation()
                var followed = false
                Live.changes(LiveTopic.SESSION_REQUESTS).collect {
                    followed = true
                    val pending = attempt { coreFlow("push.pending") { c, id -> c.pendingSessionRequests(id) } }.getOrDefault(emptyList())
                    for (request in pending) handleRequest(context, request.requestId, request.expiresAt)
                }
                wait = if (followed) FOLLOW_RETRY_MS else (wait * 2).coerceAtMost(FOLLOW_RETRY_MAX_MS)
                delay(wait)
            }
        }
    }

    /**
     * VTOP rejected the password the server keeps (`credentials_rejected`): Settings flags
     * Username and password, and a notification opens Settings, where it shows on top.
     */
    internal fun rejected(context: Context, flow: String) {
        AccountState.rejected = true
        Log.info("push", "credentials rejected", flow, mapOf("notified" to "${canNotify(context)}"))
        if (!canNotify(context)) return
        context.getSystemService(NotificationManager::class.java)
            .createNotificationChannel(NotificationChannel(REJECTED, "Account", NotificationManager.IMPORTANCE_DEFAULT).apply { description = "When VTOP rejects your saved password" })
        val open = PendingIntent.getActivity(
            context, REJECTED.hashCode(),
            Intent(context, MainActivity::class.java).putExtra(MainActivity.OPEN, SETTINGS)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val n = NotificationCompat.Builder(context, REJECTED)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle("VTOP rejected your saved password")
            .setContentText("Update it in Settings, under Username and password")
            .setContentIntent(open)
            .setAutoCancel(true)
            .build()
        NotificationManagerCompat.from(context).notify(REJECTED.hashCode(), n)
    }

    internal fun canNotify(context: Context) =
        Build.VERSION.SDK_INT < 33 || ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED

    /** A session request while the app is away: tapping opens the app, which asks ([handleRequest]). */
    private fun asked(context: Context, requestId: String, notificationId: Int, expiresAtMs: Long) {
        if (!canNotify(context)) return
        context.getSystemService(NotificationManager::class.java)
            .createNotificationChannel(NotificationChannel(ASKED, "Connected apps", NotificationManager.IMPORTANCE_HIGH).apply { description = "When a browser extension or AI app asks for your VTOP session" })
        val open = PendingIntent.getActivity(
            context, notificationId,
            Intent(context, MainActivity::class.java).putExtra(MainActivity.OPEN, MainActivity.OPEN_SESSION_REQUEST)
                .putExtra(MainActivity.ITEM, requestId)
                .putExtra(MainActivity.EXPIRES_AT, expiresAtMs)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val timeoutMs = (expiresAtMs - System.currentTimeMillis()).coerceIn(1_000L, ASKED_TIMEOUT_MS)
        val n = NotificationCompat.Builder(context, ASKED)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle("An app wants your VTOP data")
            .setContentText("Tap to allow")
            .setContentIntent(open)
            .setTimeoutAfter(timeoutMs)
            .setAutoCancel(true)
            .build()
        NotificationManagerCompat.from(context).notify(notificationId, n)
    }

    private fun cancelNotification(context: Context, notificationId: Int) {
        NotificationManagerCompat.from(context).cancel(notificationId)
    }

    internal fun send(token: String) {
        AppScope.launch {
            flow("push.token") { f ->
                val result = attempt { withContext(Dispatchers.IO) { AppCore.await().setPushToken(f, token) } }
                Log.info("push", "token", f, mapOf("sent" to "${result.isSuccess}"))
            }
        }
    }
}
