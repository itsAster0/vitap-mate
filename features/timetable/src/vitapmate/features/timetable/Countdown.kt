package vitapmate.features.timetable

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import vitapmate.ui.nums
import vitapmate.ui.rememberNow

/**
 * "Ends in 12:05" to today's [targetMinute] (India time). Ticks once a second on its own, so only
 * this text recomposes, not the agenda.
 */
@Composable
fun Countdown(targetMinute: Int, prefix: String, modifier: Modifier = Modifier) {
    val seconds = (targetMinute * 60 - istSecond(rememberNow(1_000).value)).coerceAtLeast(0)
    val h = seconds / 3600
    val m = seconds % 3600 / 60
    val s = seconds % 60
    val value = if (h > 0) "$h:%02d:%02d".format(m, s) else "$m:%02d".format(s)
    Text(
        buildAnnotatedString {
            append("$prefix ")
            withStyle(SpanStyle(fontWeight = FontWeight.W700)) { append(value) }
        },
        modifier.testTag("timetable.countdown").clearAndSetSemantics { contentDescription = "$prefix ${if (h > 0) "$h hours " else ""}$m minutes" },
        maxLines = 1,
        style = MaterialTheme.typography.labelLarge.nums(),
        color = MaterialTheme.colorScheme.primary,
    )
}
