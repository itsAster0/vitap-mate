package vitapmate.features.timetable

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import vitapmate.core.AttendanceCourse
import vitapmate.core.DayClass
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip
import vitapmate.ui.clock
import vitapmate.ui.compactMinutes
import vitapmate.ui.dayLabel
import vitapmate.ui.formatMinutes
import vitapmate.ui.nums
import vitapmate.ui.rememberNow

/**
 * The day's focus: the class under way (with a countdown to its end, a wavy progress line and
 * what's next), or the next one (a countdown to it on today, "First class" before the day
 * starts). Neutral cards; the live one is outlined in the accent.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun FocusCard(d: AgendaDay, today: Int, attendance: List<AttendanceCourse>, h24: Boolean, onOpen: (DayClass) -> Unit) {
    val c = d.current ?: d.next ?: return
    val s = c.slot
    val live = d.current != null
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    // Neutral, a step up from the other cards; the live class is outlined in the accent.
    val container = if (d.isToday) scheme.surfaceContainerHigh else scheme.surfaceContainerLow
    val content = scheme.onSurface
    val secondary = scheme.onSurfaceVariant
    val labFat = c in d.labFats
    Card(
        onClick = { onOpen(c) },
        modifier = Modifier.fillMaxWidth().testTag("timetable.focus"),
        border = if (live) BorderStroke(1.5.dp, scheme.primary.copy(alpha = 0.4f)) else null,
        shape = MaterialTheme.shapes.extraLargeIncreased,
        colors = CardDefaults.cardColors(containerColor = container, contentColor = content),
    ) {
        Column(Modifier.padding(20.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                val label = when {
                    live -> "Now"
                    !d.isToday -> "First class · ${dayLabel(d.day.day, today)}"
                    d.beforeFirst -> "First class"
                    else -> "Next"
                }
                StatusChip(label, tones.accentTone)
                Spacer(Modifier.weight(1f))
                if (d.isToday) Countdown(if (live) s.end else s.start, if (live) "Ends in" else "Starts in")
            }
            Spacer(Modifier.height(14.dp))
            Row(verticalAlignment = Alignment.Top) {
                Column(Modifier.weight(1f)) {
                    Text(s.name, style = MaterialTheme.typography.headlineSmallEmphasized, maxLines = 3, overflow = TextOverflow.Ellipsis)
                    Spacer(Modifier.height(4.dp))
                    Text(
                        "${clock(s.start, h24)} – ${clock(s.end, h24)} · ${formatMinutes(s.end - s.start)}",
                        style = MaterialTheme.typography.bodyMedium.nums(),
                        color = secondary,
                    )
                    if (s.faculty.isNotBlank()) {
                        Text(s.faculty, style = MaterialTheme.typography.bodyMedium, color = secondary, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    }
                    val last = d.isLastClass(c)
                    val clash = c in d.clashes
                    if (labFat || last || clash) {
                        Spacer(Modifier.height(10.dp))
                        ClassTags(labFat = labFat, lastClass = last, clash = clash)
                    }
                    // A Lab FAT doesn't count towards attendance.
                    val record = if (labFat) null else attendance.forSlot(s)
                    if (record != null) {
                        Spacer(Modifier.height(10.dp))
                        AttendancePill(record)
                    }
                }
                Spacer(Modifier.width(12.dp))
                RoomTile(
                    s.room,
                    s.block,
                    fill = if (live) tones.accentTone.subtle else scheme.surfaceContainerHighest,
                    onFill = if (live) tones.accentTone.onSubtle else scheme.onSurface,
                )
            }
            if (live) {
                Spacer(Modifier.height(16.dp))
                // Read in the draw lambda only: the line moves each second without recomposing.
                val now = rememberNow(1_000)
                LinearWavyProgressIndicator(
                    progress = { ((istSecond(now.value) / 60f - s.start) / (s.end - s.start)).coerceIn(0f, 1f) },
                    modifier = Modifier.fillMaxWidth(),
                    color = scheme.primary,
                )
                d.next?.let { n -> UpNext(n, d.minuteNow, h24) { onOpen(n) } }
            }
        }
    }
}

/** The class after the current one, one line under a divider, in the focus card's own colours. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun UpNext(c: DayClass, minuteNow: Int, h24: Boolean, onOpen: () -> Unit) {
    val gap = c.slot.start - minuteNow
    Spacer(Modifier.height(12.dp))
    HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
    Row(
        Modifier
            .fillMaxWidth()
            .clip(MaterialTheme.shapes.medium)
            .clickable(onClickLabel = "Open ${c.slot.name}", onClick = onOpen)
            .testTag("timetable.focus.next")
            .padding(top = 12.dp, bottom = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text("Up next", style = MaterialTheme.typography.labelMedium)
            Text(c.slot.name, style = MaterialTheme.typography.titleSmallEmphasized, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text("${clock(c.slot.start, h24)} · Room ${c.slot.room}", style = MaterialTheme.typography.bodySmall.nums(), maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        Spacer(Modifier.width(8.dp))
        Text(if (gap <= 0) "now" else "in ${compactMinutes(gap)}", style = MaterialTheme.typography.labelLargeEmphasized.nums())
    }
}
