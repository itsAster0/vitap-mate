package vitapmate.features.calendar

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import vitapmate.core.AcademicCalendar
import vitapmate.ui.AppTheme
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/**
 * Two attached tiles: the class days left, large, and the next holiday when there is one. Read
 * out as one line: "5 class days left · Next holiday Sat 7 Nov, Deepavali".
 */
@Composable
fun Outlook(cal: AcademicCalendar, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val days = if (cal.daysLeft == 0) "No class days left" else "${cal.daysLeft} class ${if (cal.daysLeft == 1) "day" else "days"} left"
    val said = days + (cal.nextHoliday?.let { h -> " · Next holiday ${formatDay(h.day, "EEE d MMM")}" + if (h.title != "Holiday") ", ${h.title}" else "" } ?: "")
    val label = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold)
    val tile = RoundedCornerShape(6.dp)
    Row(
        modifier.fillMaxWidth().height(IntrinsicSize.Min).clip(MaterialTheme.shapes.extraLarge)
            .testTag("calendar.outlook").clearAndSetSemantics { contentDescription = said },
        horizontalArrangement = Arrangement.spacedBy(3.dp),
    ) {
        val left = tones.accentTone
        Column(Modifier.weight(1f).fillMaxHeight().background(left.subtle, tile).padding(horizontal = 16.dp, vertical = 14.dp)) {
            if (cal.daysLeft == 0) {
                Text(days, style = MaterialTheme.typography.titleLargeEmphasized, color = scheme.onSurface)
            } else {
                Text("${cal.daysLeft}", style = MaterialTheme.typography.displayMediumEmphasized.nums(fontSize = 46.sp, lineHeight = 46.sp), color = scheme.onSurface)
                Text(
                    "class ${if (cal.daysLeft == 1) "day" else "days"} left",
                    Modifier.padding(top = 6.dp),
                    style = label,
                    color = left.onSubtle.copy(alpha = 0.8f),
                )
            }
        }
        cal.nextHoliday?.let { h ->
            val right = tones.success
            Column(Modifier.weight(1f).fillMaxHeight().background(right.subtle, tile).padding(horizontal = 16.dp, vertical = 14.dp)) {
                Text("Next holiday", style = label, color = right.onSubtle.copy(alpha = 0.8f))
                Text(
                    formatDay(h.day, "EEE d MMM"),
                    Modifier.padding(top = 4.dp),
                    style = MaterialTheme.typography.titleLargeEmphasized.nums(fontSize = 19.sp, fontWeight = FontWeight.ExtraBold),
                    color = scheme.onSurface,
                )
                if (h.title != "Holiday") {
                    Text(h.title, Modifier.padding(top = 2.dp), style = MaterialTheme.typography.bodyMedium, color = right.onSubtle)
                }
            }
        }
    }
}
