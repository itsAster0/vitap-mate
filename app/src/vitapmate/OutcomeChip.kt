package vitapmate

import androidx.compose.runtime.Composable
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip

/** A flow's outcome: ok, an error kind, cancelled, or running (no outcome yet). */
@Composable
fun OutcomeChip(outcome: String?) {
    val p = AppTheme.palette
    val tone = when (outcome) {
        null -> p.accentTone
        "ok" -> p.success
        "cancelled" -> p.warning
        else -> p.danger
    }
    StatusChip(outcome ?: "running", tone)
}
