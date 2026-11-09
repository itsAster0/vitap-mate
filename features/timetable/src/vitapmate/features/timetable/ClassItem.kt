package vitapmate.features.timetable

import androidx.compose.runtime.setValue
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
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
 * the code, where it is ([Location]) with the attendance ring beside it, and any tags, and on the right only its live state ("Now", "in 25m", done) or, once the day is
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
                    Column(Modifier.width(58.dp), horizontalAlignment = Alignment.Start) {
                        Text(clock(slot.start, h24), Modifier.fitDown(), maxLines = 1, softWrap = false, style = MaterialTheme.typography.labelLargeEmphasized.nums(), color = text)
                        Text(clock(slot.end, h24), Modifier.fitDown(), maxLines = 1, softWrap = false, style = MaterialTheme.typography.bodySmall.nums(), color = scheme.onSurfaceVariant)
                        // How long until it starts (or ends), under the times so the name keeps the width.
                        Countdown(state, slot, minuteNow, showPosted)
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
                    // Where it is, then how the course stands: colour only on the ring.
                    val where = slot.room.isNotBlank() || slot.block.isNotBlank()
                    val attendance = record?.takeIf { !labFat }
                    if (where || attendance != null) {
                        Spacer(Modifier.height(8.dp))
                        // One row: the room, then the ring.
                        Row(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
                            if (where) Location(slot.room, slot.block, if (labFat) tones.warning.base else tone.base, muted)
                            if (attendance != null) AttendancePill(attendance, Modifier.testTag("timetable.attendance"))
                        }
                    }
                    if (labFat || lastClass || clash || note != null) {
                        Spacer(Modifier.height(8.dp))
                        ClassTags(labFat = labFat, lastClass = lastClass, clash = clash, note = note)
                    }
                }
            },
            trailingContent = { Trailing(state, slot, minuteNow, showPosted, posted) },
        ) {
            Text(slot.name, maxLines = 2, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.titleSmallEmphasized, color = text)
        }
    }
    item()
}

/**
 * Where the class is, as one token: the room in tabular figures (what you look for on the way),
 * its block beside it, muted, and a pin in the rail's colour. Neutral fill, so the row stays quiet.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Location(room: String, block: String, pin: androidx.compose.ui.graphics.Color, muted: Boolean) {
    val scheme = MaterialTheme.colorScheme
    Row(
        Modifier
            .background(scheme.surfaceContainerHighest, RoundedCornerShape(8.dp))
            .padding(start = 6.dp, end = 8.dp, top = 3.dp, bottom = 3.dp)
            .testTag("timetable.location"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(Lucide.MapPin, null, Modifier.size(13.dp), tint = if (muted) scheme.onSurfaceVariant else pin)
        if (room.isNotBlank()) {
            Spacer(Modifier.width(4.dp))
            Text(room, maxLines = 1, softWrap = false, style = MaterialTheme.typography.labelLargeEmphasized.nums(), color = if (muted) scheme.onSurfaceVariant else scheme.onSurface)
        }
        if (block.isNotBlank()) {
            Spacer(Modifier.width(if (room.isNotBlank()) 6.dp else 4.dp))
            Text(block, maxLines = 1, softWrap = false, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        }
    }
}

private val RailInset = 14.dp

/** Under the times: "in 25m" (the next class in the accent), "12m left", or the length on other days. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Countdown(state: ClassState, slot: ClassSlot, minuteNow: Int?, showPosted: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val (text, accent) = when {
        showPosted || state == ClassState.Done && minuteNow != null -> return
        state == ClassState.Now && minuteNow != null -> "${compactMinutes(slot.end - minuteNow)} left" to true
        state == ClassState.Next && minuteNow != null -> "in ${compactMinutes(slot.start - minuteNow)}" to true
        state == ClassState.Later && minuteNow != null -> "in ${compactMinutes(slot.start - minuteNow)}" to false
        else -> compactMinutes(slot.end - slot.start) to false
    }
    Text(
        text,
        Modifier.padding(top = 4.dp).fitDown(),
        maxLines = 1,
        softWrap = false,
        style = (if (accent) MaterialTheme.typography.labelMediumEmphasized else MaterialTheme.typography.labelMedium).nums(),
        color = if (accent) scheme.primary else scheme.onSurfaceVariant,
    )
}

/** The right-hand state: posted attendance after the day, else Now or done. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Trailing(state: ClassState, slot: ClassSlot, minuteNow: Int?, showPosted: Boolean, posted: ClassStatus?) {
    val tones = AppTheme.tones
    val scheme = MaterialTheme.colorScheme
    when {
        showPosted -> when (posted) {
            ClassStatus.PRESENT -> StatusChip("Present", tones.success)
            ClassStatus.ON_DUTY -> StatusChip("On duty", tones.accentTone)
            ClassStatus.ABSENT -> StatusChip("Absent", tones.danger)
            ClassStatus.OTHER -> StatusChip("Marked", tones.accentTone)
            null -> Text("Not posted", style = MaterialTheme.typography.labelMedium.nums(), color = scheme.onSurfaceVariant)
        }
        state == ClassState.Now && minuteNow != null -> StatusChip("Now", tones.accentTone)
        state == ClassState.Done && minuteNow != null -> Icon(Lucide.Check, "Done", Modifier.size(18.dp), tint = scheme.onSurfaceVariant)
    }
}
