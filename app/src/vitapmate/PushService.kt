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
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.LiveTopic
import kotlinx.coroutines.flow.collectLatest
import vitapmate.data.AccountState
import vitapmate.data.AppCore
import vitapmate.data.AppScope
import vitapmate.data.Foreground
import vitapmate.data.Live
import vitapmate.data.Settings
import vitapmate.data.coreFlow
import vitapmate.data.Tunable
import vitapmate.data.problemOf
import vitapmate.data.toasts
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.showToast

/**
 * FCM: a new token goes to the server with this phone's device; a `vtop_cookie_request` wakes
 * the app when it isn't on screen (on screen, [Push.follow]'s live query answers it); a
 * `credentials_rejected` flags the account and notifies ([Push.rejected]); other messages are
 * only logged. Tokens and message contents are never logged.
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
        val requestId = data["requestId"] ?: return
        // On screen, the live query has it already.
        if (Foreground.shown) return
        // FCM keeps the process for this call only, so it runs here (a binder thread).
        Settings.load(applicationContext)
        AppCore.start(applicationContext)
        runBlocking { Push.answer(applicationContext, requestId) }
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
     * Answers a key's session request (`vtop_cookie_request`) with this phone's live VTOP session,
     * signing in again if it expired, within [Tunable.SessionAnswer] plus the time the student has
     * to answer the cookie dialog ([Tunable.CookieAsk]). Only on screen: with the app away (and
     * not [tapped] from [asked]'s notification) nothing is sent, and the edge's wait runs out.
     * Flow `push.session_request`; no cookies in the log.
     */
    suspend fun answer(context: Context, requestId: String, tapped: Boolean = false) = flow("push.session_request") { f ->
        if (!tapped && !Foreground.shown) {
            asked(context, requestId)
            Log.info("push", "session request asked by notification", f)
            return@flow
        }
        if (!answered.add(requestId)) {
            Log.info("push", "session request already answered", f)
            return@flow
        }
        val limit = (Settings.tunable(Tunable.SessionAnswer) + Settings.tunable(Tunable.CookieAsk)) * 1000L
        val result = runCatching {
            withTimeoutOrNull(limit) { withContext(Dispatchers.IO) { AppCore.await().answerSessionRequest(f, requestId, null) } }
                ?: withContext(Dispatchers.IO) { AppCore.await().answerSessionRequest(f, requestId, "The phone took too long to sign in to VTOP.") }
        }
        Log.info("push", "session request", f, mapOf("answered" to "${result.isSuccess}", "error" to (result.exceptionOrNull()?.let { it::class.simpleName } ?: "-")))
        // On screen, say so: the app waiting on it only times out.
        result.exceptionOrNull()?.let { e ->
            val problem = problemOf(e, f)
            if (Foreground.shown && problem.toasts) showToast("Couldn't answer a connected app", problem.message)
        }
    }

    private const val ASKED = "connected_apps"
    private const val REJECTED = "account"
    private const val FOLLOW_RETRY_MS = 30_000L
    private const val FOLLOW_RETRY_MAX_MS = 5 * 60_000L
    private const val IDENTITY_CHECK_MS = 5_000L

    /** Requests answered (or being answered) in this process: the live query and a tap can both bring one. */
    private val answered: MutableSet<String> = ConcurrentHashMap.newKeySet()

    /**
     * Answers each session request waiting on this phone as the server lists it
     * (`sessions:pending`, live) while the app runs; FCM only wakes an app that isn't. Flow
     * `push.pending` per change. Without a server identity (it proves only when a server feature
     * needs it) the watch can't follow, so the phone looks again on its own every
     * [IDENTITY_CHECK_MS] and follows as soon as it has one. A watch that ends anyway is tried
     * again with back-off. Flow `push.follow` while on screen.
     */
    suspend fun follow(context: Context): Nothing = Foreground.shownFlow.collectLatest { shown ->
        // Off screen, FCM brings requests and the socket may close.
        if (!shown) return@collectLatest
        flow("push.follow") { f ->
            var wait = FOLLOW_RETRY_MS
            while (true) {
                while (!runCatching { withContext(Dispatchers.IO) { AppCore.await().hasServerIdentity(f) } }.getOrDefault(false)) delay(IDENTITY_CHECK_MS)
                var followed = false
                Live.changes(LiveTopic.SESSION_REQUESTS).collect {
                    followed = true
                    val pending = runCatching { coreFlow("push.pending") { c, id -> c.pendingSessionRequests(id) } }.getOrDefault(emptyList())
                    for (request in pending) if (request.requestId !in answered) AppScope.launch { answer(context, request.requestId) }
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

    /** A session request while the app is away: tapping opens the app, which asks ([answer]). */
    private fun asked(context: Context, requestId: String) {
        if (!canNotify(context)) return
        context.getSystemService(NotificationManager::class.java)
            .createNotificationChannel(NotificationChannel(ASKED, "Connected apps", NotificationManager.IMPORTANCE_HIGH).apply { description = "When a browser extension or AI app asks for your VTOP session" })
        val open = PendingIntent.getActivity(
            context, ASKED.hashCode(),
            Intent(context, MainActivity::class.java).putExtra(MainActivity.OPEN, MainActivity.OPEN_SESSION_REQUEST)
                .putExtra(MainActivity.ITEM, requestId)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        val n = NotificationCompat.Builder(context, ASKED)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle("An app wants your VTOP data")
            .setContentText("Tap to allow")
            .setContentIntent(open)
            .setTimeoutAfter(60_000)
            .setAutoCancel(true)
            .build()
        NotificationManagerCompat.from(context).notify(ASKED.hashCode(), n)
    }

    internal fun send(token: String) {
        AppScope.launch {
            flow("push.token") { f ->
                val result = runCatching { withContext(Dispatchers.IO) { AppCore.await().setPushToken(f, token) } }
                Log.info("push", "token", f, mapOf("sent" to "${result.isSuccess}"))
            }
        }
    }
}
