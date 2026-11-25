package vitapmate.features.outing

import androidx.compose.runtime.saveable.Saver

/** Where an apply sheet is: being filled in, the request shown to confirm, or sent to VTOP. */
internal sealed interface ApplyPhase {
    /** [failure]: why the last Review or send didn't go through (VTOP's refusal), shown above Review. */
    data class Editing(val failure: String? = null) : ApplyPhase

    data object Confirming : ApplyPhase

    /** On its way; the sheet stays open until VTOP answers. */
    data object Sending : ApplyPhase

    companion object {
        /** Only a failure outlives a recreated sheet: an open confirm or a send in flight doesn't. */
        val Saver: Saver<ApplyPhase, String> = Saver({ (it as? Editing)?.failure }, ::Editing)
    }
}
