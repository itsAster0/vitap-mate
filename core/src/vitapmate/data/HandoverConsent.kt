package vitapmate.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.ConsentAnswer
import vitapmate.core.ConsentAsk
import vitapmate.core.HandoverAsk
import vitapmate.core.HandoverSheetPhase
import vitapmate.core.HandoverStatus
import vitapmate.core.KeyKind

/**
 * UI mirror for Rust's connected-app handover approve sheet (`consent.rs`) and handover status
 * stream (for notifications): Rust owns the sheet phase, timeouts and app-left denial.
 */
object HandoverConsent {
    /** The sheet's phase; [ask] is what it shows. */
    sealed interface Sheet {
        val id: ULong
        val flow: String
        val ask: HandoverAsk

        class Asking(override val id: ULong, override val ask: HandoverAsk, override val flow: String) : Sheet
        data class Sending(override val id: ULong, override val ask: HandoverAsk, override val flow: String) : Sheet
        data class Sent(override val id: ULong, override val ask: HandoverAsk, override val flow: String) : Sheet
        data class Failed(override val id: ULong, override val ask: HandoverAsk, override val flow: String, val reason: String) : Sheet
        data class Expired(override val id: ULong, override val ask: HandoverAsk, override val flow: String) : Sheet
    }

    private val sheetState = MutableStateFlow<Sheet?>(null)

    /** The sheet to show, mirrored from Rust. */
    val sheet: StateFlow<Sheet?> = sheetState.asStateFlow()

    private val statusFlow = MutableSharedFlow<Pair<String, HandoverStatus>>(extraBufferCapacity = 16)

    /** Each approved answer's progress (request id, status), for its notification. */
    val statuses: SharedFlow<Pair<String, HandoverStatus>> = statusFlow.asSharedFlow()

    internal fun onConsent(ask: ConsentAsk) {
        sheetState.value = when (ask) {
            is ConsentAsk.Handover -> when (val phase = ask.phase) {
                HandoverSheetPhase.Asking -> Sheet.Asking(ask.id, ask.ask, ask.flow)
                HandoverSheetPhase.Sending -> Sheet.Sending(ask.id, ask.ask, ask.flow)
                HandoverSheetPhase.Sent -> Sheet.Sent(ask.id, ask.ask, ask.flow)
                is HandoverSheetPhase.Failed -> Sheet.Failed(ask.id, ask.ask, ask.flow, phase.reason)
                HandoverSheetPhase.Expired -> Sheet.Expired(ask.id, ask.ask, ask.flow)
            }
            else -> null
        }
    }

    internal fun onStatus(requestId: String, status: HandoverStatus) {
        statusFlow.tryEmit(requestId to status)
    }

    /** Approve or Deny on the open sheet. */
    fun answer(approve: Boolean) {
        val asking = sheetState.value as? Sheet.Asking ?: return
        val mapped = if (approve) ConsentAnswer.APPROVE else ConsentAnswer.DENY
        AppScope.launch {
            attempt { withContext(Dispatchers.IO) { AppCore.await().consentAnswer(asking.flow, asking.id, mapped) } }
        }
    }

    /** Done, Close or a swipe: an open question is Deny; a finished sheet goes. */
    fun close() {
        val current = sheetState.value ?: return
        val mapped = if (current is Sheet.Asking) ConsentAnswer.DENY else ConsentAnswer.CANCEL
        AppScope.launch {
            attempt { withContext(Dispatchers.IO) { AppCore.await().consentAnswer(current.flow, current.id, mapped) } }
        }
    }
}

/** "Chrome on MacBook", or what asked when the server didn't say. */
val HandoverAsk.who: String get() = asker?.keyLabel ?: "A connected app"

/** Whether an AI app (MCP key) asked. */
val HandoverAsk.byAiApp: Boolean get() = asker?.via == KeyKind.MCP
