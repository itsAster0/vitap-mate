package vitapmate.features.timetable

import vitapmate.ui.OpenHaptic
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
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

    @Composable
    fun line(icon: ImageVector, text: String) = Row(Modifier.padding(vertical = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(36.dp).background(scheme.surfaceContainerHigh, MaterialTheme.shapes.medium), Alignment.Center) {
            Icon(icon, null, Modifier.size(18.dp), tint = scheme.onSurfaceVariant)
        }
        Spacer(Modifier.width(14.dp))
        Text(text, Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge.nums())
    }

    OpenHaptic()
    ModalBottomSheet(onDismissRequest = onDismiss, modifier = Modifier.testTag("timetable.sheet"), containerColor = scheme.surfaceContainerLow) {
        Column(Modifier.fillMaxWidth().padding(start = 24.dp, end = 24.dp, bottom = 24.dp)) {
            val lab = slot.kind == ClassKind.LAB
            Text(
                listOf(if (labFat) "Lab FAT" else if (lab) "Lab" else "Theory", slot.code).filter { it.isNotBlank() }.joinToString(" · "),
                style = MaterialTheme.typography.labelLarge,
                color = when {
                    labFat -> AppTheme.tones.warning.onSubtle
                    lab -> AppTheme.tones.lab.onSubtle
                    else -> AppTheme.tones.accentTone.onSubtle
                },
            )
            Spacer(Modifier.height(6.dp))
            Text(slot.name, style = MaterialTheme.typography.headlineSmallEmphasized)
            Spacer(Modifier.height(12.dp))
            line(Lucide.Clock, "${WEEKDAYS.getOrElse(slot.day) { "" }}, ${clock(slot.start, h24)} – ${clock(slot.end, h24)} · ${formatMinutes(slot.end - slot.start)}")
            line(Lucide.MapPin, listOf("Room ${slot.room}", slot.block).filter { it.isNotBlank() }.joinToString(" · "))
            if (slot.faculty.isNotBlank()) line(Lucide.User, slot.faculty)
            line(Lucide.Hash, "${slot.code} · Slot ${slot.slot}")
            slot.credits?.takeIf { it > 0 }?.let { c ->
                val n = if (c % 1.0 == 0.0) "%.0f".format(c) else c.toString()
                line(Lucide.Award, "$n ${if (n == "1") "credit" else "credits"}")
            }
            Spacer(Modifier.height(16.dp))
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
                modifier = Modifier.size(52.dp),
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
