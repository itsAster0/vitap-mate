package vitapmate.features.timetable

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Undo2
import vitapmate.core.DayClass
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/**
 * The day's name and [note] (the calendar's word on the day, as a chip) over "5 November ·
 * 9:00 AM – 5:30 PM" on one line, with a Today button when another day is shown. The heading is
 * the same height every day, so the list below doesn't shift between days.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun DayHeading(day: Int, isToday: Boolean, slots: List<DayClass>, note: String?, onToday: () -> Unit, h24: Boolean) {
    val summary = listOfNotNull(
        formatDay(day, "d MMMM"),
        if (slots.isEmpty()) null else "${clock(slots.minOf { it.slot.start }, h24)} – ${clock(slots.maxOf { it.slot.end }, h24)}",
    ).joinToString("  ·  ")
    Row(verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    if (isToday) "Today" else formatDay(day, "EEEE"),
                    style = MaterialTheme.typography.displaySmallEmphasized,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                if (note != null) {
                    Spacer(Modifier.width(8.dp))
                    val labFat = note == "Lab FAT"
                    StatusChip(
                        note,
                        if (labFat) AppTheme.tones.warning else AppTheme.tones.accentTone,
                        Modifier.weight(1f, fill = false).testTag("timetable.day.note"),
                        icon = if (labFat) Lucide.FlaskConical else null,
                    )
                }
            }
            Text(
                if (isToday) "${formatDay(day, "EEEE")}, $summary" else summary,
                Modifier.fitDown(),
                style = MaterialTheme.typography.bodyMedium.nums(),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                maxLines = 1,
                softWrap = false,
            )
        }
        AnimatedVisibility(!isToday, enter = fadeIn() + scaleIn(initialScale = 0.8f), exit = fadeOut() + scaleOut(targetScale = 0.8f)) {
            Row {
                Spacer(Modifier.width(8.dp))
                FilledTonalButton(onToday, modifier = Modifier.testTag("timetable.today"), shapes = ButtonDefaults.shapes()) {
                    Icon(Lucide.Undo2, null, Modifier.size(16.dp))
                    Spacer(Modifier.width(6.dp))
                    Text("Today")
                }
            }
        }
    }
}
