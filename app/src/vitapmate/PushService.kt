package vitapmate

import android.content.Context
import com.google.firebase.messaging.FirebaseMessaging
import com.google.firebase.messaging.FirebaseMessagingService
import com.google.firebase.messaging.RemoteMessage
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.data.AppCore
import vitapmate.data.Settings
import vitapmate.data.Tunable
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * FCM: a new token goes to the edge with this phone's device (signed, see `device_sig`); a
 * `vtop_cookie_request` is answered at once ([Push.answer]); other messages are nudges to read the
 * phone's event log. Tokens and message contents are never logged.
 * Log src `push`.
 */
class PushService : FirebaseMessagingService() {
    // Deprecated in firebase-messaging 26 for register()/onRegistered, but that doesn't hand
    // back the token on later starts; these still work.
    @Deprecated("FCM 26")
    @Suppress("DEPRECATION")
    override fun onNewToken(token: String) = Push.send(applicationContext, token)

    override fun onMessageReceived(message: RemoteMessage) {
        val data = message.data
        val kind = data["kind"] ?: "-"
        if (kind != "vtop_cookie_request") {
            MainScope().launch { flow("push.message") { f -> Log.info("push", "message", f, mapOf("kind" to kind)) } }
            return
        }
        val requestId = data["requestId"] ?: return
        val callbackUrl = data["callbackUrl"] ?: return
        // FCM keeps the process for this call only, so it runs here (a binder thread) under the
        // Developer timeout; when that runs out the edge is told so instead.
        Settings.load(applicationContext)
        AppCore.start(applicationContext)
        runBlocking { Push.answer(requestId, callbackUrl) }
    }
}

object Push {
    /** Sends the current token once the app has started (FCM may not call [PushService.onNewToken] again). */
    @Suppress("DEPRECATION")
    fun start(context: Context) {
        runCatching { FirebaseMessaging.getInstance().token.addOnSuccessListener { send(context, it) } }
            .onFailure { Log.warn("push", "no firebase: ${it::class.simpleName}") }
    }

    /**
     * Answers a key's session request (`vtop_cookie_request`) with this phone's live VTOP session,
     * signing in again if it expired, within [Tunable.SessionAnswer]; never shown to the student.
     * Flow `push.session_request`; no cookies in the log.
     */
    suspend fun answer(requestId: String, callbackUrl: String) = flow("push.session_request") { f ->
        val limit = Settings.tunable(Tunable.SessionAnswer) * 1000L
        val result = runCatching {
            withTimeoutOrNull(limit) { withContext(Dispatchers.IO) { AppCore.await().answerSessionRequest(f, requestId, callbackUrl, null) } }
                ?: withContext(Dispatchers.IO) { AppCore.await().answerSessionRequest(f, requestId, callbackUrl, "The phone took too long to sign in to VTOP.") }
        }
        Log.info("push", "session request", f, mapOf("answered" to "${result.isSuccess}", "error" to (result.exceptionOrNull()?.let { it::class.simpleName } ?: "-")))
    }

    internal fun send(context: Context, token: String) {
        MainScope().launch {
            flow("push.token") { f ->
                val result = runCatching { withContext(Dispatchers.IO) { AppCore.await().setPushToken(f, token) } }
                Log.info("push", "token", f, mapOf("sent" to "${result.isSuccess}"))
            }
        }
    }
}
