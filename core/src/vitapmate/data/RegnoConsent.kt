package vitapmate.data

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.RegnoAnswer
import vitapmate.observability.Log

/**
 * Rust's "Share your registration number?" sheet (`regno.rs`, asked through [CookieConsent]'s
 * gate): [ask] puts a request in [pending] for the shell's sheet and waits for Share or Not now.
 * Rust asks one at a time; a newer ask closes an older sheet. No answer within
 * [Tunable.CookieAsk], swiping it away or the app leaving the screen is Dismissed, which changes
 * nothing. Log src `ui.regno`.
 */
object RegnoConsent {
    class Request(val flow: String) {
        internal val answer = CompletableDeferred<RegnoAnswer>()
    }

    private val pendingState = MutableStateFlow<Request?>(null)

    /** The sheet to show. Thread-safe for updates from Rust / IO threads. */
    val pending: StateFlow<Request?> = pendingState.asStateFlow()

    /** How long the sheet waits; tests shorten it. */
    internal var waitMs = { Settings.tunable(Tunable.CookieAsk) * 1000L }

    suspend fun ask(flow: String): RegnoAnswer {
        if (!Foreground.shown) return RegnoAnswer.DISMISSED
        val request = Request(flow)
        val older = synchronized(this) { pendingState.value.also { pendingState.value = request } }
        older?.let { answer(it, RegnoAnswer.DISMISSED) }
        Log.info("ui.regno", "shown", flow)
        try {
            return withTimeoutOrNull(waitMs()) { request.answer.await() } ?: RegnoAnswer.DISMISSED
        } finally {
            // Answered, timed out or cancelled (Rust dropped the call): the sheet goes.
            answer(request, RegnoAnswer.DISMISSED)
        }
    }

    /** The student's answer (or Dismissed); the sheet goes. */
    fun answer(request: Request, answer: RegnoAnswer) {
        synchronized(this) { if (pendingState.value === request) pendingState.value = null }
        if (request.answer.complete(answer)) {
            Log.info("ui.regno", "answered", request.flow, mapOf("answer" to answer.name.lowercase()))
        }
    }

    /** The app left the screen: an open sheet is Dismissed. */
    internal fun left() = pendingState.value?.let { answer(it, RegnoAnswer.DISMISSED) }
}
