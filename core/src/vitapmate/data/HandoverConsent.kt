package vitapmate.data

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.HandoverAsk
import vitapmate.core.HandoverStatus
import vitapmate.core.KeyKind
import vitapmate.observability.Log

/**
 * Rust's approve sheet for a connected app's request (`consent::allowed_handover`, asked through
 * [CookieConsent]'s gate) and how an approved answer goes (`HandoverStatus`). [ask] shows the
 * sheet and waits for Approve or Deny; after Approve the sheet stays, sending, until Rust says
 * the server took the session or why not. No answer within [Tunable.CookieAsk] or before the
 * request expires is Deny and shows it expired; swiping it away or the app leaving the screen is
 * Deny. [statuses] carries every status, for the notification. Log src `ui.handover`; the
 * address is never logged.
 */
object HandoverConsent {
    /** The sheet's phase; [ask] is what it shows. */
    sealed interface Sheet {
        val ask: HandoverAsk

        class Asking(override val ask: HandoverAsk, val flow: String) : Sheet {
            internal val answer = CompletableDeferred<Boolean>()
        }

        data class Sending(override val ask: HandoverAsk) : Sheet
        data class Sent(override val ask: HandoverAsk) : Sheet
        data class Failed(override val ask: HandoverAsk, val reason: String) : Sheet
        data class Expired(override val ask: HandoverAsk) : Sheet
    }

    private val sheetState = MutableStateFlow<Sheet?>(null)

    /** The sheet to show. Thread-safe for updates from Rust / IO threads. */
    val sheet: StateFlow<Sheet?> = sheetState.asStateFlow()

    private val statusFlow = MutableSharedFlow<Pair<String, HandoverStatus>>(extraBufferCapacity = 16)

    /** Each approved answer's progress (request id, status), for its notification. */
    val statuses: SharedFlow<Pair<String, HandoverStatus>> = statusFlow.asSharedFlow()

    /** How long the sheet waits at most; tests shorten it. */
    internal var waitMs = { Settings.tunable(Tunable.CookieAsk) * 1000L }

    suspend fun ask(flow: String, ask: HandoverAsk): Boolean {
        if (!Foreground.shown) return false
        val asking = Sheet.Asking(ask, flow)
        val older = synchronized(this) { sheetState.value.also { sheetState.value = asking } }
        (older as? Sheet.Asking)?.answer?.complete(false)
        Log.info(
            "ui.handover",
            "shown",
            flow,
            mapOf("via" to (ask.asker?.via?.name?.lowercase() ?: "-"), "ip_seen" to "${ask.asker?.ip?.seen}"),
        )
        val limit = ask.expiresAt?.let { minOf(waitMs(), it - System.currentTimeMillis()) } ?: waitMs()
        var approved: Boolean? = null
        try {
            approved = withTimeoutOrNull(limit.coerceAtLeast(0)) { asking.answer.await() }
            return approved == true
        } finally {
            // Approved: sending until Rust says how it went. Denied or cancelled (Rust dropped the
            // call): gone. Timed out: says it expired.
            synchronized(this) {
                if (sheetState.value === asking) {
                    sheetState.value = when (approved) {
                        true -> Sheet.Sending(ask)
                        false -> null
                        null -> if (asking.answer.isCompleted) null else Sheet.Expired(ask)
                    }
                }
            }
            asking.answer.complete(false)
        }
    }

    /** Approve or Deny on the open sheet. */
    fun answer(approve: Boolean) {
        val asking = sheetState.value as? Sheet.Asking ?: return
        if (asking.answer.complete(approve)) {
            Log.info("ui.handover", "answered", asking.flow, mapOf("approve" to "$approve"))
        }
    }

    /** Done, Close or a swipe: an open question is Deny; a finished sheet goes. */
    fun close() {
        if (sheetState.value is Sheet.Asking) return answer(false)
        synchronized(this) { if (sheetState.value !is Sheet.Asking) sheetState.value = null }
    }

    fun status(flow: String, requestId: String, status: HandoverStatus) {
        statusFlow.tryEmit(requestId to status)
        Log.info("ui.handover", "status", flow, mapOf("status" to (status::class.simpleName ?: "-").lowercase()))
        synchronized(this) {
            val current = sheetState.value ?: return
            if (current is Sheet.Asking || current.ask.requestId != requestId) return
            sheetState.value = when (status) {
                is HandoverStatus.Fetching -> Sheet.Sending(current.ask)
                is HandoverStatus.Sent -> Sheet.Sent(current.ask)
                is HandoverStatus.Failed -> Sheet.Failed(current.ask, status.reason)
            }
        }
    }

    /** The app left the screen: an open question is Deny (a sending sheet stays for its result). */
    internal fun left() = (sheetState.value as? Sheet.Asking)?.answer?.complete(false)
}

/** "Chrome on MacBook", or what asked when the server didn't say. */
val HandoverAsk.who: String get() = asker?.keyLabel ?: "A connected app"

/** Whether an AI app (MCP key) asked. */
val HandoverAsk.byAiApp: Boolean get() = asker?.via == KeyKind.MCP
