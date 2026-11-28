package vitapmate.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.ConsentAnswer
import vitapmate.core.ConsentAsk
import vitapmate.core.RegnoAnswer

/**
 * UI mirror for Rust's "Share your registration number?" sheet (`regno.rs` / `consent.rs`):
 * Rust owns pending asks, timeouts and app-left dismissal, pushing through [AppEventsBridge].
 */
object RegnoConsent {
    class Request(val id: ULong, val flow: String)

    private val pendingState = MutableStateFlow<Request?>(null)

    /** The active registration-number sheet, mirrored from Rust. */
    val pending: StateFlow<Request?> = pendingState.asStateFlow()

    internal fun onConsent(ask: ConsentAsk) {
        pendingState.value = when (ask) {
            is ConsentAsk.Regno -> Request(ask.id, ask.flow)
            else -> null
        }
    }

    /** Sends the student's answer (or Dismissed) to Rust. */
    fun answer(request: Request, answer: RegnoAnswer) {
        val mapped = when (answer) {
            RegnoAnswer.SHARE -> ConsentAnswer.SHARE
            RegnoAnswer.NOT_NOW -> ConsentAnswer.NOT_NOW
            RegnoAnswer.DISMISSED -> ConsentAnswer.CANCEL
        }
        AppScope.launch {
            attempt { withContext(Dispatchers.IO) { AppCore.await().consentAnswer(request.flow, request.id, mapped) } }
        }
    }
}
