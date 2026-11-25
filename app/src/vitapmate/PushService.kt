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
import java.util.concurrent.atomic.AtomicBoolean
import vitapmate.core.HandoverAsker
import vitapmate.core.HandoverEffect
import vitapmate.core.HandoverEvent
import vitapmate.core.HandoverStatus
import vitapmate.core.KeyKind
import vitapmate.data.HandoverConsent
import vitapmate.core.LiveTopic
import vitapmate.data.AppCore
import vitapmate.data.orNull
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
 * FCM: a new token goes to the server with this phone's device; a `vtop_cookie_request` goes to
 * Rust ([Push.handover]: notifying off screen, answering on screen alongside [Push.follow]); a
 * `credentials_rejected` flags the account and notifies ([Push.rejected]);
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
        if (Foreground.shown) {
            // Don't drop FCM while on screen: Push.follow may be in backoff or the app may
            // background before the live query delivers it.
            AppScope.launch { Push.handover(applicationContext, HandoverEvent.Pushed(data, foreground = true)) }
            return
        }
        // FCM keeps the process for this call only, so it runs here (a binder thread).
        Settings.load(applicationContext)
        AppCore.start(applicationContext)
        runBlocking { Push.handover(applicationContext, HandoverEvent.Pushed(data, foreground = false)) }
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
     * Tells Rust of a connected app's session request (FCM, `sessions:pending`, a notification
     * tap) or the app coming on screen, and does what it says: post or take down a notification,
     * or answer within [Tunable.SessionAnswer] plus [Tunable.CookieAsk]. Flow
     * `push.session_request`; no cookies in the log.
     */
    suspend fun handover(context: Context, event: HandoverEvent) {
        val app = context.applicationContext
        flow("push.session_request") { f ->
            val effects = attempt { withContext(Dispatchers.IO) { AppCore.await().handover(f, event) } }.getOrDefault(emptyList())
            for (effect in effects) when (effect) {
                is HandoverEffect.Notify -> asked(app, effect.requestId, effect.notificationId, effect.expiresAt, effect.asker)
                is HandoverEffect.Cancel -> NotificationManagerCompat.from(app).cancel(effect.notificationId)
                is HandoverEffect.Answer -> AppScope.launch { answer(effect.requestId, effect.attempt) }
            }
        }
    }

    /** Answers session requests whose notification went up while the app was away. */
    fun onForeground(context: Context) {
        AppScope.launch { handover(context, HandoverEvent.Shown) }
    }

    private suspend fun answer(requestId: String, attemptId: ULong) =
        flow("push.session_request") { f ->
            val limit = Settings.tunable(Tunable.SessionAnswer) + Settings.tunable(Tunable.CookieAsk)
            val result = attempt { withContext(Dispatchers.IO) { AppCore.await().answerHandover(f, requestId, attemptId, limit.toUInt()) } }
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
    private const val SENT_TIMEOUT_MS = 10_000L
    /** Sending notifications' ids, apart from the requests' own (Rust's board) and the account's. */
    private const val SENDING_BASE = 0x5000_0000
    private const val SENDING_MASK = 0x0fff_ffff
    private const val FOLLOW_RETRY_MS = 30_000L
    private const val FOLLOW_RETRY_MAX_MS = 5 * 60_000L

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
        handover(context, HandoverEvent.Shown)
        flow("push.follow") { f ->
            var wait = FOLLOW_RETRY_MS
            while (true) {
                // No core, no identity ever: nothing to follow until the app leaves the screen.
                if (attempt { AppCore.await().waitServerIdentity(f) }.isFailure) awaitCancellation()
                var followed = false
                Live.changes(LiveTopic.SESSION_REQUESTS).collect {
                    followed = true
                    val pending = attempt { coreFlow("push.pending") { c, id -> c.pendingSessionRequests(id) } }.getOrDefault(emptyList())
                    for (request in pending) handover(context, HandoverEvent.Listed(request.requestId, request.expiresAt, Foreground.shown, request.asker))
                }
                wait = if (followed) FOLLOW_RETRY_MS else (wait * 2).coerceAtMost(FOLLOW_RETRY_MAX_MS)
                delay(wait)
            }
        }
    }

    /**
     * VTOP rejected the password the server keeps (`credentials_rejected`): Settings flags
     * Your account, and a notification opens Settings, where it shows on top.
     */
    internal suspend fun rejected(context: Context, flow: String) {
        // Kept with the account by Rust, so Settings still flags it after a restart.
        orNull { withContext(Dispatchers.IO) { AppCore.await().passwordRejectedOnServer(flow) } }
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
            .setContentText("Update it in Settings, under Your account")
            .setContentIntent(open)
            .setAutoCancel(true)
            .build()
        NotificationManagerCompat.from(context).notify(REJECTED.hashCode(), n)
    }

    private val watching = AtomicBoolean(false)

    /**
     * Each approved answer's progress ([HandoverConsent.statuses]) as one notification per
     * request: sending, then signed in or why not. Statuses of requests not approved on this
     * phone are ignored. Once per process.
     */
    fun watchHandovers(context: Context) {
        if (!watching.compareAndSet(false, true)) return
        val app = context.applicationContext
        AppScope.launch {
            val approved = mutableMapOf<String, HandoverAsker?>()
            HandoverConsent.statuses.collect { (requestId, status) ->
                if (status is HandoverStatus.Fetching) approved[requestId] = status.asker
                if (requestId !in approved) return@collect
                val who = approved[requestId]?.keyLabel ?: "A connected app"
                if (status !is HandoverStatus.Fetching) approved.remove(requestId)
                sending(app, requestId, who, status)
            }
        }
    }

    private fun sending(context: Context, requestId: String, who: String, status: HandoverStatus) {
        if (!canNotify(context)) return
        connectedAppsChannel(context)
        val id = SENDING_BASE or (requestId.hashCode() and SENDING_MASK)
        val open = PendingIntent.getActivity(
            context, id,
            Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val n = NotificationCompat.Builder(context, ASKED)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentIntent(open)
            .setOnlyAlertOnce(true)
        when (status) {
            is HandoverStatus.Fetching -> n.setContentTitle("Signing in $who")
                .setContentText("Getting your VTOP session and sending it")
                .setOngoing(true)
                .setSilent(true)
                .setProgress(0, 0, true)
                .setTimeoutAfter(ASKED_TIMEOUT_MS * 2)
            is HandoverStatus.Sent -> n.setContentTitle("$who is signed in")
                .setContentText("Your phone sent it a VTOP session")
                .setSilent(true)
                .setAutoCancel(true)
                .setTimeoutAfter(SENT_TIMEOUT_MS)
            is HandoverStatus.Failed -> n.setContentTitle("Couldn't sign in $who")
                .setContentText(status.reason)
                .setAutoCancel(true)
        }
        NotificationManagerCompat.from(context).notify(id, n.build())
    }

    internal fun canNotify(context: Context) =
        Build.VERSION.SDK_INT < 33 || ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED

    /** A session request while the app is away: tapping opens the app, which asks ([HandoverEvent.Tapped]). */
    private fun connectedAppsChannel(context: Context) = context.getSystemService(NotificationManager::class.java)
        .createNotificationChannel(NotificationChannel(ASKED, "Connected apps", NotificationManager.IMPORTANCE_HIGH).apply { description = "When a browser extension or AI app asks for your VTOP session" })

    private fun asked(context: Context, requestId: String, notificationId: Int, expiresAtMs: Long, asker: HandoverAsker?) {
        if (!canNotify(context)) return
        connectedAppsChannel(context)
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
            .setContentTitle(asker?.let { "${it.keyLabel} wants to sign in" } ?: "An app wants your VTOP data")
            .setContentText(
                when {
                    asker?.ip != null -> "From ${asker.ip!!.address}${if (asker.ip!!.seen) "" else " (new IP)"} · tap to review"
                    asker?.via == KeyKind.MCP -> "An AI app · tap to review"
                    else -> "Tap to review"
                },
            )
            .setContentIntent(open)
            .setTimeoutAfter(timeoutMs)
            .setAutoCancel(true)
            .build()
        NotificationManagerCompat.from(context).notify(notificationId, n)
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
