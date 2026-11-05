package vitapmate.features.timetable

import androidx.compose.runtime.setValue
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.layout.onSizeChanged
import com.composables.icons.lucide.FlaskConical
import androidx.compose.ui.draw.clip
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.ListItemShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SegmentedListItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MapPin
import vitapmate.core.AttendanceCourse
import vitapmate.core.ClassKind
import vitapmate.core.ClassSlot
import vitapmate.core.ClassStatus
import vitapmate.data.Settings
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip
import vitapmate.ui.clock
import vitapmate.ui.compactMinutes
import vitapmate.ui.nums

/**
 * One class of the day as a segmented list item: start and end, a lab/theory bar, then the name,
 * code and room and the attendance stacked, and on the right only its live state ("Now", "in 25m", done) or, once the day is
 * over, what VTOP posted ([showPosted]; null [posted] is not posted yet), over the attendance.
 * Back-to-back classes share a group ([shapes]); free time splits groups. Tap opens the sheet.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ClassItem(
    slot: ClassSlot,
    state: ClassState,
    shapes: ListItemShapes,
    record: AttendanceCourse?,
    minuteNow: Int?,
    labFat: Boolean,
    lastClass: Boolean,
    clash: Boolean,
    note: String?,
    showPosted: Boolean,
    posted: ClassStatus?,
    h24: Boolean,
    onOpen: () -> Unit,
) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val live = state == ClassState.Now
    // Over (and not recapped) or not meeting: the text steps back to the muted role.
    val muted = state == ClassState.Done && !showPosted || note != null
    val text = if (muted) scheme.onSurfaceVariant else scheme.onSurface
    val tone = if (slot.kind == ClassKind.LAB) tones.lab else tones.accentTone
    val container = if (live) scheme.surfaceContainerHigh else scheme.surfaceContainerLow
    var itemHeight by remember { mutableIntStateOf(0) }
    val item: @Composable () -> Unit = {
        SegmentedListItem(
            onClick = onOpen,
            shapes = shapes,
            // Live: a step up from the other rows, outlined in the accent; no saturated fill.
            modifier = Modifier.testTag(classTag(slot)).onSizeChanged { itemHeight = it.height }.then(if (live) Modifier.border(1.5.dp, scheme.primary.copy(alpha = 0.4f), shapes.shape) else Modifier),
            colors = ListItemDefaults.segmentedColors(containerColor = container),
            onLongClickLabel = null,
            leadingContent = {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.width(58.dp), horizontalAlignment = Alignment.End) {
                        Text(clock(slot.start, h24), Modifier.fitDown(end = true), maxLines = 1, softWrap = false, style = MaterialTheme.typography.labelLargeEmphasized.nums(), color = text)
                        Text(clock(slot.end, h24), Modifier.fitDown(end = true), maxLines = 1, softWrap = false, style = MaterialTheme.typography.bodySmall.nums(), color = scheme.onSurfaceVariant)
                    }
                    Spacer(Modifier.width(12.dp))
                    // The rail runs the item's height, with the same gap above and below.
                    val rail = with(LocalDensity.current) { (itemHeight.toDp() - RailInset * 2).coerceAtLeast(40.dp) }
                    Box(Modifier.width(if (live) 5.dp else 4.dp).height(rail).background(if (labFat) tones.warning.base else tone.base, RoundedCornerShape(2.5.dp)))
                }
            },
            supportingContent = {
                Column {
                    Text(slot.code, maxLines = 1, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
                    if (lastClass || clash || note != null) {
                        Spacer(Modifier.height(6.dp))
                        ClassTags(lastClass = lastClass, clash = clash, note = note)
                    }
                }
            },
            trailingContent = { Trailing(state, slot, minuteNow, showPosted, posted) },
        ) {
            Text(slot.name, maxLines = 2, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.titleSmallEmphasized, color = text)
        }
    }
    // Every card has a band at its foot: where (and a Lab FAT) on the left, the attendance on the
    // right.
    // The band takes the rail's colour: lab, theory, or amber on a Lab FAT.
    val bandTone = if (labFat) tones.warning else tone
    val band = bandTone.subtle
    val onBand = bandTone.onSubtle
    Column(Modifier.clip(shapes.shape).background(container)) {
        item()
        // Only what applies, spaced evenly (Flutter's spaceEvenly): Lab FAT, room, block, and the
        // attendance pill (not on a Lab FAT: nothing to skip).
        Row(
            Modifier.fillMaxWidth().background(band).padding(horizontal = 8.dp, vertical = 8.dp).testTag(if (labFat) "timetable.labfat" else "timetable.band"),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.SpaceEvenly,
        ) {
            val label = MaterialTheme.typography.labelMedium
            @Composable
            fun Piece(text: String, icon: androidx.compose.ui.graphics.vector.ImageVector? = null) =
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    icon?.let { Icon(it, null, Modifier.size(14.dp), tint = onBand) }
                    Text(text, maxLines = 1, softWrap = false, style = label, color = onBand)
                }
            if (labFat) Piece("Lab FAT", Lucide.FlaskConical)
            if (slot.room.isNotBlank()) Piece("Room ${slot.room}", Lucide.MapPin.takeIf { !labFat })
            if (slot.block.isNotBlank()) Piece(slot.block)
            if (record != null && !labFat) {
                AttendancePill(
                    record,
                    Modifier.clip(CircleShape).background(onBand.copy(alpha = 0.12f)).padding(start = 6.dp, end = 10.dp, top = 3.dp, bottom = 3.dp),
                )
            }
        }
    }
}

private val RailInset = 14.dp

/** The right-hand state: posted attendance after the day, else Now / in 25m / done / the length. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Trailing(state: ClassState, slot: ClassSlot, minuteNow: Int?, showPosted: Boolean, posted: ClassStatus?) {
    val tones = AppTheme.tones
    val scheme = MaterialTheme.colorScheme
    val muted = MaterialTheme.typography.labelMedium.nums()
    when {
        showPosted -> when (posted) {
            ClassStatus.PRESENT -> StatusChip("Present", tones.success)
            ClassStatus.ON_DUTY -> StatusChip("On duty", tones.accentTone)
            ClassStatus.ABSENT -> StatusChip("Absent", tones.danger)
            ClassStatus.OTHER -> StatusChip("Marked", tones.accentTone)
            null -> Text("Not posted", style = muted, color = scheme.onSurfaceVariant)
        }
        state == ClassState.Now && minuteNow != null -> Column(horizontalAlignment = Alignment.End) {
            StatusChip("Now", tones.accentTone)
            Text("${compactMinutes(slot.end - minuteNow)} left", style = MaterialTheme.typography.labelMediumEmphasized.nums(), color = scheme.primary)
        }
        state == ClassState.Next && minuteNow != null -> Text("in ${compactMinutes(slot.start - minuteNow)}", style = MaterialTheme.typography.labelLargeEmphasized.nums(), color = scheme.primary)
        state == ClassState.Done && minuteNow != null -> Icon(Lucide.Check, "Done", Modifier.size(18.dp), tint = scheme.onSurfaceVariant)
        state == ClassState.Later && minuteNow != null -> Text("in ${compactMinutes(slot.start - minuteNow)}", style = muted, color = scheme.onSurfaceVariant)
        else -> Text(compactMinutes(slot.end - slot.start), style = muted, color = scheme.onSurfaceVariant)
    }
}
