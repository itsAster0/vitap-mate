package vitapmate.features.timetable

import vitapmate.ui.OpenHaptic
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CornerSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Award
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.Hash
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MapPin
import com.composables.icons.lucide.User
import kotlin.math.roundToInt
import vitapmate.core.AttendanceCourse
import vitapmate.core.ClassKind
import vitapmate.core.ClassSlot
import vitapmate.data.Settings
import vitapmate.ui.AppTheme
import vitapmate.ui.clock
import vitapmate.ui.formatMinutes
import vitapmate.ui.nums

private val WEEKDAYS = listOf("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday")

/**
 * A class's details from the bottom: kind, name, time, room, faculty, slot and credits, then its
 * attendance (none for a Lab FAT; a note when VTOP hasn't counted a class yet).
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ClassSheet(slot: ClassSlot, record: AttendanceCourse?, labFat: Boolean, h24: Boolean, onDismiss: () -> Unit) {
    val scheme = MaterialTheme.colorScheme

    // One tile of the group: its label with the icon over the value.
    @Composable
    fun tile(icon: ImageVector, label: String, text: String, shape: Shape, modifier: Modifier = Modifier) =
        Column(modifier.background(scheme.surfaceContainerHigh, shape).padding(horizontal = 14.dp, vertical = 12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(icon, null, Modifier.size(14.dp), tint = scheme.onSurfaceVariant)
                Spacer(Modifier.width(6.dp))
                Text(label, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
            }
            Spacer(Modifier.height(3.dp))
            Text(text, style = MaterialTheme.typography.titleSmallEmphasized.nums())
        }

    OpenHaptic()
    ModalBottomSheet(onDismissRequest = onDismiss, modifier = Modifier.testTag("timetable.sheet"), containerColor = scheme.surfaceContainerLow) {
        Column(Modifier.fillMaxWidth().padding(start = 24.dp, end = 24.dp, bottom = 24.dp)) {
            val lab = slot.kind == ClassKind.LAB
            val tone = if (labFat) AppTheme.tones.warning else if (lab) AppTheme.tones.lab else AppTheme.tones.accentTone
            Row(verticalAlignment = Alignment.Top) {
                Column(Modifier.weight(1f)) {
                    Text(
                        listOf(if (labFat) "Lab FAT" else if (lab) "Lab" else "Theory", slot.code).filter { it.isNotBlank() }.joinToString(" · "),
                        style = MaterialTheme.typography.labelLarge,
                        color = tone.onSubtle,
                    )
                    Spacer(Modifier.height(6.dp))
                    Text(slot.name, style = MaterialTheme.typography.headlineSmallEmphasized)
                }
                Spacer(Modifier.width(12.dp))
                RoomTile(slot.room, slot.block, tone.subtle, tone.onSubtle, size = 80.dp)
            }
            Spacer(Modifier.height(18.dp))
            // A connected group: large corners outside, small ones between tiles.
            val big = CornerSize(20.dp)
            val small = CornerSize(6.dp)
            val credits = slot.credits?.takeIf { it > 0 }?.let { c ->
                val n = if (c % 1.0 == 0.0) "%.0f".format(c) else c.toString()
                "$n ${if (n == "1") "credit" else "credits"}"
            }
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                val wide = Modifier.fillMaxWidth()
                tile(Lucide.Clock, "When", "${WEEKDAYS.getOrElse(slot.day) { "" }}, ${clock(slot.start, h24)} – ${clock(slot.end, h24)} · ${formatMinutes(slot.end - slot.start)}", RoundedCornerShape(big, big, small, small), wide)
                tile(Lucide.MapPin, "Where", listOf("Room ${slot.room}", slot.block).filter { it.isNotBlank() }.joinToString(" · "), RoundedCornerShape(small), wide)
                if (slot.faculty.isNotBlank()) tile(Lucide.User, "Faculty", slot.faculty, RoundedCornerShape(small), wide)
                Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    val code = "${slot.code} · Slot ${slot.slot}"
                    if (credits == null) {
                        tile(Lucide.Hash, "Code and slot", code, RoundedCornerShape(small, small, big, big), wide)
                    } else {
                        tile(Lucide.Hash, "Code and slot", code, RoundedCornerShape(small, small, small, big), Modifier.weight(3f).fillMaxHeight())
                        tile(Lucide.Award, "Credits", credits, RoundedCornerShape(small, small, big, small), Modifier.weight(2f).fillMaxHeight())
                    }
                }
            }
            Spacer(Modifier.height(14.dp))
            when {
                labFat -> Note("Lab FAT: this session doesn't count towards attendance.")
                record == null -> Note("No attendance for this course yet. It shows here once VTOP counts a class.")
                else -> AttendanceBox(record)
            }
        }
    }
}

@Composable
private fun Note(text: String) = Text(
    text,
    Modifier.fillMaxWidth().background(MaterialTheme.colorScheme.surfaceContainerHigh, MaterialTheme.shapes.large).padding(16.dp).testTag("timetable.sheet.note"),
    style = MaterialTheme.typography.bodyMedium,
    color = MaterialTheme.colorScheme.onSurfaceVariant,
)

/** The course's attendance: a ring with the percentage, VTOP's "x of y", and the advice when the setting is on. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun AttendanceBox(record: AttendanceCourse) {
    val tone = toneOf(record)
    val skips = Settings.showSkipAdvice
    // TODO(attendance): tap to open the course's attendance details once a shared route exists.
    Row(
        Modifier.fillMaxWidth().background(tone.subtle, MaterialTheme.shapes.extraLarge).padding(16.dp).testTag("timetable.sheet.attendance"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(contentAlignment = Alignment.Center) {
            CircularProgressIndicator(
                progress = { (record.percent / 100).toFloat() },
                modifier = Modifier.size(60.dp),
                color = tone.base,
                trackColor = MaterialTheme.colorScheme.surfaceContainerHighest,
                strokeWidth = 5.dp,
            )
            Text("${record.percent.roundToInt()}", style = MaterialTheme.typography.labelLargeEmphasized.nums(), color = tone.onSubtle)
        }
        Spacer(Modifier.width(16.dp))
        Column(Modifier.weight(1f)) {
            Text(if (skips) record.advice else "Attendance", style = MaterialTheme.typography.titleSmallEmphasized, color = tone.onSubtle)
            Text("${record.attended} of ${record.total} attended", style = MaterialTheme.typography.bodyMedium.nums(), color = tone.onSubtle)
        }
    }
}
