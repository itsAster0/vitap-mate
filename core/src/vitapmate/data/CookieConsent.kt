package vitapmate.data

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.CookieAnswer
import vitapmate.core.CookieGate
import vitapmate.core.CookieKind
import vitapmate.observability.Log
import vitapmate.observability.Unattended

/**
 * Rust's question before the VTOP session goes to our servers (`consent.rs`): a flow is attended
 * while the app is shown and it isn't a background run; [ask] puts a request in [pending] for the
 * shell's dialog and waits for Send or Cancel. Asks of one action at once share one dialog. No
 * answer within [Tunable.CookieAsk], or the app leaving the screen, is Cancel. Log src `ui.consent`.
 */
object CookieConsent : CookieGate {
    class Request(val flow: String, val kind: CookieKind, val action: String, val server: String) {
        internal val answer = CompletableDeferred<CookieAnswer>()
        internal var waiters = 0
    }

    private val pendingState = MutableStateFlow<List<Request>>(emptyList())

    /** The dialogs to show, oldest first. Thread-safe for updates from Rust / IO threads. */
    val pending: StateFlow<List<Request>> = pendingState.asStateFlow()

    private val no = CookieAnswer(send = false, remember = false)

    /** How long a dialog waits; tests shorten it. */
    internal var waitMs = { Settings.tunable(Tunable.CookieAsk) * 1000L }

    override fun attended(flow: String): Boolean = Foreground.shown && flow !in Unattended.ids

    override suspend fun ask(flow: String, kind: CookieKind, action: String, server: String): CookieAnswer {
        if (!Foreground.shown) return no // Left between `attended` and here: nobody would see it.
        // Only the very action the dialog names joins it; anything else waits for its own.
        val request = synchronized(this) {
            val current = pendingState.value
            val same = current.firstOrNull { it.kind == kind && it.action == action && it.server == server }
            (same ?: Request(flow, kind, action, server).also {
                pendingState.value = current + it
                Log.info("ui.consent", "shown", flow, mapOf("kind" to kind.name.lowercase()))
            }).also { it.waiters++ }
        }
        try {
            return withTimeoutOrNull(waitMs()) { request.answer.await() } ?: no.also { answer(request, it) }
        } finally {
            // Cancelled (Rust dropped the call): the last one waiting takes the dialog away.
            if (synchronized(this) { --request.waiters == 0 }) answer(request, no)
        }
    }

    /** The student's answer (or Cancel); the dialog goes. */
    fun answer(request: Request, answer: CookieAnswer) {
        synchronized(this) { pendingState.value = pendingState.value - request }
        if (request.answer.complete(answer)) {
            Log.info("ui.consent", "answered", request.flow, mapOf("kind" to request.kind.name.lowercase(), "send" to "${answer.send}", "remember" to "${answer.remember}"))
        }
    }

    /** The app left the screen: every open dialog is Cancel. */
    internal fun left() = synchronized(this) { pendingState.value }.forEach { answer(it, no) }
}
