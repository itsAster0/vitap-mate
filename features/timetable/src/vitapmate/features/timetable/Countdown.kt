package vitapmate.features.timetable

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.unit.dp
import vitapmate.ui.nums
import vitapmate.ui.rememberNow

/**
 * "12:05 until it ends" (or starts, unless [ends]) to today's [targetMinute] (India time), large.
 * Ticks once a second on its own, so only this text recomposes, not the agenda.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun Countdown(targetMinute: Int, ends: Boolean, modifier: Modifier = Modifier) {
    val seconds = (targetMinute * 60 - istSecond(rememberNow(1_000).value)).coerceAtLeast(0)
    val h = seconds / 3600
    val m = seconds % 3600 / 60
    val s = seconds % 60
    val value = if (h > 0) "$h:%02d:%02d".format(m, s) else "$m:%02d".format(s)
    val prefix = if (ends) "Ends in" else "Starts in"
    Row(
        modifier.testTag("timetable.countdown").clearAndSetSemantics { contentDescription = "$prefix ${if (h > 0) "$h hours " else ""}$m minutes" },
        verticalAlignment = Alignment.Bottom,
    ) {
        Text(value, Modifier.alignByBaseline(), maxLines = 1, style = MaterialTheme.typography.displayMediumEmphasized.nums())
        Spacer(Modifier.width(8.dp))
        Text(if (ends) "until it ends" else "until it starts", Modifier.alignByBaseline(), maxLines = 1, style = MaterialTheme.typography.titleSmall)
    }
}
