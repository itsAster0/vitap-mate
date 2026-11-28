package vitapmate.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.ConsentAnswer
import vitapmate.core.ConsentAsk
import vitapmate.core.CookieAnswer
import vitapmate.core.CookieKind
import vitapmate.core.CookiePayload

/**
 * UI mirror for Rust's cookie consent dialog (`consent.rs`): Rust owns pending prompts, coalescing,
 * timeouts and app-left cancellation, pushing the active prompt through [AppEventsBridge].
 */
object CookieConsent {
    class Request(
        val id: ULong,
        val flow: String,
        val kind: CookieKind,
        val payload: CookiePayload,
        val action: String,
        val server: String,
    )

    private val pendingState = MutableStateFlow<List<Request>>(emptyList())

    /** The active cookie dialog (at most one), mirrored from Rust. */
    val pending: StateFlow<List<Request>> = pendingState.asStateFlow()

    internal fun onConsent(ask: ConsentAsk) {
        pendingState.value = when (ask) {
            is ConsentAsk.Cookie -> listOf(Request(ask.id, ask.flow, ask.kind, ask.payload, ask.action, ask.server))
            else -> emptyList()
        }
    }

    /** Sends the student's answer (or Cancel) to Rust. */
    fun answer(request: Request, answer: CookieAnswer) {
        val mapped = when (answer) {
            is CookieAnswer.Allow -> if (answer.remember) ConsentAnswer.DONT_ASK_AGAIN else ConsentAnswer.SEND
            CookieAnswer.Deny -> ConsentAnswer.CANCEL
        }
        AppScope.launch {
            attempt { withContext(Dispatchers.IO) { AppCore.await().consentAnswer(request.flow, request.id, mapped) } }
        }
    }
}
