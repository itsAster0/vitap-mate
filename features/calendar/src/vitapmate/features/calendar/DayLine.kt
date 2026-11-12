package vitapmate.features.calendar

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.core.CalendarDay
import vitapmate.ui.AppTheme
import vitapmate.ui.Dot
import vitapmate.ui.StatusChip
import vitapmate.ui.formatDay
import vitapmate.ui.neutralTone
import vitapmate.ui.nums

/**
 * The tapped day and, under it, what VTOP lists for it: "Thu 5 Nov" / "Instructional Day (LAB FAT)",
 * on its mark's fill (neutral without one), with how far off it is from [today].
 */
@Composable
fun DayLine(day: CalendarDay, today: Int, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val tone = day.mark?.let { markTone(it) }
    val away = when (val d = day.day - today) {
        0 -> "Today"
        1 -> "Tomorrow"
        -1 -> "1 day ago"
        else -> if (d > 0) "In $d days" else "${-d} days ago"
    }
    Row(
        modifier.fillMaxWidth().background(tone?.subtle ?: scheme.surfaceContainerHigh, RoundedCornerShape(16.dp)).padding(horizontal = 14.dp, vertical = 10.dp).testTag("calendar.selected"),
        horizontalArrangement = Arrangement.spacedBy(10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        if (tone != null) Dot(size = 8.dp, color = if (hollow(day.mark!!)) Color.Transparent else tone.base, ring = if (hollow(day.mark!!)) tone.base else null)
        Column(Modifier.weight(1f)) {
            Text(formatDay(day.day, "EEE d MMM"), style = MaterialTheme.typography.labelLarge.nums(color = scheme.onSurface))
            Text(day.label.ifBlank { "Not on the calendar" }, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
        }
        StatusChip(away, (tone ?: neutralTone).copy(subtle = AppTheme.colors.card), Modifier.testTag("calendar.selected.away"))
    }
}
