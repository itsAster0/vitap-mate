package vitapmate.features.timetable

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import vitapmate.core.AttendanceCourse
import vitapmate.core.DayClass
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip
import vitapmate.ui.Tone
import vitapmate.ui.clock
import vitapmate.ui.compactMinutes
import vitapmate.ui.dayLabel
import vitapmate.ui.formatMinutes
import vitapmate.ui.nums
import vitapmate.ui.rememberNow

/**
 * The day's focus: the class under way (solid accent, with a large countdown to its end, a wavy
 * progress line and what's next), or the next one (the quiet accent fill, a countdown to it on
 * today, "First class" before the day starts). Another day's first class sits on a neutral card.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun FocusCard(d: AgendaDay, today: Int, attendance: List<AttendanceCourse>, h24: Boolean, onOpen: (DayClass) -> Unit) {
    val c = d.current ?: d.next ?: return
    val s = c.slot
    val live = d.current != null
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val container = when {
        live -> tones.accent
        d.isToday -> tones.accentTone.subtle
        else -> scheme.surfaceContainerLow
    }
    val content = when {
        live -> tones.onAccent
        d.isToday -> tones.accentTone.onSubtle
        else -> scheme.onSurface
    }
    val secondary = if (d.isToday) content.copy(alpha = 0.82f) else scheme.onSurfaceVariant
    val labFat = c in d.labFats
    Card(
        onClick = { onOpen(c) },
        modifier = Modifier.fillMaxWidth().testTag("timetable.focus"),
        shape = MaterialTheme.shapes.extraLargeIncreased,
        colors = CardDefaults.cardColors(containerColor = container, contentColor = content),
    ) {
        Column(Modifier.padding(20.dp)) {
            val label = when {
                live -> "Now"
                !d.isToday -> "First class · ${dayLabel(d.day.day, today)}"
                d.beforeFirst -> "First class"
                else -> "Next"
            }
            // On the solid fill the chip is drawn inverted; on the quiet fill, solid.
            if (live) StatusChip(label, Tone(tones.accent, tones.onAccent, tones.accent)) else StatusChip(label, tones.accentTone, solid = d.isToday)
            Spacer(Modifier.height(10.dp))
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
                        // On the accent fills the pill sits on the surface, so its tone stays readable.
                        AttendancePill(record, if (d.isToday) Modifier.background(scheme.surface, CircleShape).padding(start = 6.dp, end = 9.dp, top = 3.dp, bottom = 3.dp) else Modifier)
                    }
                }
                Spacer(Modifier.width(12.dp))
                RoomTile(
                    s.room,
                    s.block,
                    fill = when {
                        live -> tones.onAccent
                        d.isToday -> scheme.surface
                        else -> scheme.surfaceContainerHighest
                    },
                    onFill = if (live) tones.accent else content,
                    size = if (live) 92.dp else 84.dp,
                    blockColor = secondary,
                )
            }
            if (d.isToday) {
                Spacer(Modifier.height(16.dp))
                Countdown(if (live) s.end else s.start, ends = live)
            }
            if (live) {
                Spacer(Modifier.height(12.dp))
                // Read in the draw lambda only: the line moves each second without recomposing.
                val now = rememberNow(1_000)
                LinearWavyProgressIndicator(
                    progress = { ((istSecond(now.value) / 60f - s.start) / (s.end - s.start)).coerceIn(0f, 1f) },
                    modifier = Modifier.fillMaxWidth(),
                    color = content,
                    trackColor = content.copy(alpha = 0.24f),
                )
                d.next?.let { n -> UpNext(n, d.minuteNow, h24, content) { onOpen(n) } }
            }
        }
    }
}

/** The class after the current one, in an inset panel of the focus card's own colours. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun UpNext(c: DayClass, minuteNow: Int, h24: Boolean, content: Color, onOpen: () -> Unit) {
    val gap = c.slot.start - minuteNow
    Spacer(Modifier.height(14.dp))
    Row(
        Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(20.dp))
            .background(content.copy(alpha = 0.12f))
            .clickable(onClickLabel = "Open ${c.slot.name}", onClick = onOpen)
            .testTag("timetable.focus.next")
            .padding(horizontal = 14.dp, vertical = 12.dp),
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
