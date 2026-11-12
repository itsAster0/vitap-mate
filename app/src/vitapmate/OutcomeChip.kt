package vitapmate

import androidx.compose.runtime.Composable
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip
import vitapmate.ui.Tone

/** A flow's outcome: ok, an error kind, cancelled, or running (no outcome yet). */
@Composable
fun OutcomeChip(outcome: String?) {
    StatusChip(outcome ?: "running", outcomeTone(outcome))
}

/** The tone of a flow's [outcome]: success for ok, warning for cancelled, danger for an error, accent while running. */
@Composable
fun outcomeTone(outcome: String?): Tone {
    val p = AppTheme.palette
    return when (outcome) {
        null -> p.accentTone
        "ok" -> p.success
        "cancelled" -> p.warning
        else -> p.danger
    }
}
