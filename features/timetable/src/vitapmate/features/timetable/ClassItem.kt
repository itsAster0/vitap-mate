package vitapmate.features.timetable

import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.background
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
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Lucide
import vitapmate.core.AttendanceCourse
import vitapmate.core.ClassKind
import vitapmate.core.ClassSlot
import vitapmate.core.ClassStatus
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip
import vitapmate.ui.Tone
import vitapmate.ui.clock
import vitapmate.ui.compactMinutes
import vitapmate.ui.nums

/**
 * One class of the day as a step on the timeline: start and end with the countdown, a node on the
 * rail for its state (done, now, next, later), then a segmented card with the name, the code, the
 * attendance ring, any tags and, once the day is over, what VTOP posted ([showPosted]; null
 * [posted] is not posted yet), and the room in a badge of the kind's colour. The rail runs from the
 * [first] class's node to the [last] one's, in the accent through classes that are over.
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
    first: Boolean,
    last: Boolean,
    h24: Boolean,
    onOpen: () -> Unit,
) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val live = state == ClassState.Now
    // Over (and not recapped) or not meeting: the text steps back to the muted role.
    val muted = state == ClassState.Done && !showPosted || note != null
    val text = if (muted) scheme.onSurfaceVariant else scheme.onSurface
    val tone = if (labFat) tones.warning else if (slot.kind == ClassKind.LAB) tones.lab else tones.accentTone
    val rail = if (state == ClassState.Done) tones.accent else scheme.outlineVariant
    val timeWidth = timeColumnWidth(h24)
    Row(
        Modifier.drawBehind {
            // Through the gap to the next row of the group; the free-time row draws its own.
            val top = if (first) NodeCentre.toPx() else 0f
            val bottom = if (last) NodeCentre.toPx() else size.height + ListItemDefaults.SegmentedGap.toPx()
            drawRect(rail, Offset((timeWidth + RailGap + RailX).toPx() - 1.dp.toPx(), top), Size(2.dp.toPx(), bottom - top))
        },
    ) {
        Column(Modifier.width(timeWidth).padding(top = NodeTop)) {
            Text(clock(slot.start, h24), Modifier.fitDown(), maxLines = 1, softWrap = false, style = MaterialTheme.typography.labelLargeEmphasized.nums(), color = text)
            Text(clock(slot.end, h24), Modifier.fitDown(), maxLines = 1, softWrap = false, style = MaterialTheme.typography.bodySmall.nums(), color = scheme.onSurfaceVariant)
            // How long until it starts (or ends), under the times so the name keeps the width.
            Countdown(state, slot, minuteNow, showPosted)
        }
        Spacer(Modifier.width(RailGap))
        Box(Modifier.width(Rail).padding(top = NodeTop)) { Node(state) }
        Spacer(Modifier.width(6.dp))
        SegmentedListItem(
            onClick = onOpen,
            shapes = shapes,
            modifier = Modifier.weight(1f).testTag(classTag(slot)),
            // Live: the accent's quiet fill.
            colors = ListItemDefaults.segmentedColors(containerColor = if (live) tones.accentTone.subtle else scheme.surfaceContainerLow),
            onLongClickLabel = null,
            supportingContent = {
                Column {
                    Text(slot.code, maxLines = 1, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
                    val attendance = record?.takeIf { !labFat }
                    // The ring, what VTOP posted and the tags; whatever doesn't fit wraps whole to the next line.
                    if (attendance != null || showPosted || labFat || lastClass || clash || note != null) {
                        Spacer(Modifier.height(8.dp))
                        ClassTags(labFat = labFat, lastClass = lastClass, clash = clash, note = note) {
                            if (attendance != null) AttendancePill(attendance, Modifier.testTag("timetable.attendance"))
                            if (showPosted) when (posted) {
                                ClassStatus.PRESENT -> StatusChip("Present", tones.success)
                                ClassStatus.ON_DUTY -> StatusChip("On duty", tones.accentTone)
                                ClassStatus.ABSENT -> StatusChip("Absent", tones.danger)
                                ClassStatus.OTHER -> StatusChip("Marked", tones.accentTone)
                                null -> Text("Not posted", style = MaterialTheme.typography.labelMedium.nums(), color = scheme.onSurfaceVariant)
                            }
                        }
                    }
                }
            },
            trailingContent = if (slot.room.isBlank() && slot.block.isBlank()) null else ({ RoomBadge(slot.room, slot.block, tone, live, muted) }),
        ) {
            Text(slot.name, maxLines = 2, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.titleSmallEmphasized, color = text)
        }
    }
}

/**
 * Where the class is: the room in heavy tabular figures (what you look for on the way) over its
 * block, on the kind's quiet fill; solid while live, neutral when muted.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun RoomBadge(room: String, block: String, tone: Tone, live: Boolean, muted: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val ink = if (live) scheme.surface else if (muted) scheme.onSurfaceVariant else tone.onSubtle
    Column(
        Modifier
            .size(58.dp)
            .background(if (live) tone.base else if (muted) scheme.surfaceContainerHigh else tone.subtle, RoundedCornerShape(20.dp))
            .padding(horizontal = 5.dp)
            .testTag("timetable.location"),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        if (room.isNotBlank()) {
            Text(room, Modifier.fitDown(), maxLines = 1, softWrap = false, style = MaterialTheme.typography.titleSmallEmphasized.nums(fontWeight = FontWeight.W800), color = ink)
        }
        if (block.isNotBlank()) {
            Text(block, Modifier.fitDown(), maxLines = 1, softWrap = false, style = MaterialTheme.typography.labelSmall, color = ink.copy(alpha = 0.85f))
        }
    }
}

/** The class's state on the rail: a check when done, a haloed dot now, a ring next, a small ring later. */
@Composable
private fun Node(state: ClassState) {
    val accent = AppTheme.tones.accent
    val onAccent = AppTheme.tones.onAccent
    val surface = MaterialTheme.colorScheme.surface
    val outline = MaterialTheme.colorScheme.outlineVariant
    Box(
        Modifier
            .size(NodeSize)
            .then(if (state == ClassState.Now) Modifier.semantics { contentDescription = "Now" } else Modifier)
            .drawBehind {
                val r = size.minDimension / 2
                when (state) {
                    ClassState.Done -> drawCircle(accent, r)
                    ClassState.Now -> {
                        drawCircle(surface, r + 4.dp.toPx())
                        drawCircle(accent.copy(alpha = 0.28f), r + 4.dp.toPx())
                        drawCircle(accent, r)
                        drawCircle(onAccent, 3.dp.toPx())
                    }
                    ClassState.Next -> {
                        drawCircle(surface, r)
                        drawCircle(accent, r - 1.5.dp.toPx(), style = Stroke(3.dp.toPx()))
                    }
                    ClassState.Later -> {
                        drawCircle(surface, 6.dp.toPx())
                        drawCircle(outline, 5.dp.toPx(), style = Stroke(2.dp.toPx()))
                    }
                }
            },
        contentAlignment = Alignment.Center,
    ) {
        if (state == ClassState.Done) Icon(Lucide.Check, "Done", Modifier.size(11.dp), tint = onAccent)
    }
}

/** As wide as the widest clock, the same on every row so the rail lines up; the countdown shrinks to it. */
@Composable
fun timeColumnWidth(h24: Boolean): Dp {
    val style = MaterialTheme.typography.labelLargeEmphasized.nums()
    val measurer = rememberTextMeasurer()
    return with(LocalDensity.current) { remember(h24, style, this) { measurer.measure(clock(23 * 60, h24), style).size.width.toDp() } }
}

/** The timeline's geometry: the rail column, the gap before it, the line's centre in it, and the node. */
internal val Rail = 22.dp
internal val RailGap = 6.dp
internal val RailX = 9.dp
private val NodeTop = 13.dp
private val NodeSize = 18.dp
private val NodeCentre = NodeTop + NodeSize / 2

/** Under the times: "in 25m" (the next class in the accent), "12m left", or the length on other days. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Countdown(state: ClassState, slot: ClassSlot, minuteNow: Int?, showPosted: Boolean) {
    val scheme = MaterialTheme.colorScheme
    // No space in "1h30m", so it shrinks less to the clock's width.
    fun span(m: Int) = compactMinutes(m).replace(" ", "")
    val (text, accent) = when {
        showPosted || state == ClassState.Done && minuteNow != null -> return
        state == ClassState.Now && minuteNow != null -> "${span(slot.end - minuteNow)} left" to true
        state == ClassState.Next && minuteNow != null -> "in ${span(slot.start - minuteNow)}" to true
        state == ClassState.Later && minuteNow != null -> "in ${span(slot.start - minuteNow)}" to false
        else -> span(slot.end - slot.start) to false
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
