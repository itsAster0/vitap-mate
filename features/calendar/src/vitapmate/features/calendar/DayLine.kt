package vitapmate.features.calendar

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.core.CalendarDay
import vitapmate.ui.Dot
import vitapmate.ui.Space
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/** The tapped day and what VTOP lists for it: "Thu 5 Nov · Instructional Day (LAB FAT)". */
@Composable
fun DayLine(day: CalendarDay, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val tone = day.mark?.let { markTone(it) }
    Row(modifier.fillMaxWidth().testTag("calendar.selected"), horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
        if (tone != null) Dot(size = 6.dp, color = if (hollow(day.mark!!)) Color.Transparent else tone.base, ring = if (hollow(day.mark!!)) tone.base else null)
        Text(formatDay(day.day, "EEE d MMM"), style = MaterialTheme.typography.labelLarge.nums(color = scheme.onSurface))
        Text(day.label.ifBlank { "Not on the calendar" }, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
    }
}
