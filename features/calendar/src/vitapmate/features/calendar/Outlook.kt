package vitapmate.features.calendar

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import vitapmate.core.AcademicCalendar
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/** "5 class days left · Next holiday Sat 7 Nov, Deepavali". */
@Composable
fun Outlook(cal: AcademicCalendar, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val strong = SpanStyle(fontWeight = FontWeight.SemiBold, color = scheme.onSurface)
    val text = buildAnnotatedString {
        if (cal.daysLeft == 0) {
            append("No class days left")
        } else {
            withStyle(strong) { append("${cal.daysLeft}") }
            append(" class ${if (cal.daysLeft == 1) "day" else "days"} left")
        }
        cal.nextHoliday?.let { h ->
            append("  ·  Next holiday ")
            withStyle(strong) { append(formatDay(h.day, "EEE d MMM")) }
            if (h.title != "Holiday") append(", ${h.title}")
        }
    }
    Text(text, modifier.testTag("calendar.outlook"), style = MaterialTheme.typography.bodyMedium.nums(color = scheme.onSurfaceVariant))
}
