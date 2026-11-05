package vitapmate.features.timetable

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.CornerSize
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.max
import androidx.compose.ui.unit.sp
import androidx.tracing.trace
import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.Lucide
import kotlin.math.floor
import vitapmate.core.CalendarMark
import vitapmate.core.ClassDay
import vitapmate.core.ClassKind
import vitapmate.core.ClassSlot
import vitapmate.core.TimetableDay
import vitapmate.ui.AppTheme
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PressScale
import vitapmate.ui.clock
import vitapmate.ui.isoWeekday
import vitapmate.ui.nums

private val GUTTER = 32.dp
private val HEADER = 60.dp
private val NOTE = 14.dp
private val HOUR = 80.dp
private val MIN_DAY = 108.dp
private val GRID_TOP = 10.dp
private val DAY_NAMES = listOf("Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun")

private class Block(val slot: ClassSlot, val column: Int, val labFat: Boolean)

/**
 * The week as a grid with a column per class day ([classDays], ISO weekdays, plus any working day
 * that follows another day's order), scrolling both ways: the
 * day header stays on top and the hours on the left. It opens on today at the current time
 * (India time). Blocks show code, room, time and as much of the name as fits; tap one to open it
 * ([onOpen]). Holidays are named under their date and Lab FATs tagged on their block. [window]
 * (days from this week's Monday) is null when the days are not known: no dates, no today, no now line.
 *
 * Built for scrolling: one fixed-size [Layout] places every block; the scroll offsets and the
 * clock are read only in placement and draw, so scrolling never recomposes.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun WeeklyGrid(
    classDays: List<Int>,
    slots: List<ClassSlot>,
    window: List<TimetableDay>?,
    now: State<Long>,
    h24: Boolean,
    onOpen: (ClassSlot) -> Unit,
    modifier: Modifier = Modifier,
) {
    val scheme = MaterialTheme.colorScheme
    val today by remember { derivedStateOf { istDay(now.value) } }
    val weekday = if (window == null) 0 else isoWeekday(today)
    // On Sundays the grid shows the coming week, since the current one is over.
    val dates = window?.let { if (weekday == 7) it.subList(7, 14) else it.subList(0, 7) }
    val start = slots.minOf { it.start } / 60 * 60
    val end = (slots.maxOf { it.end } + 59) / 60 * 60
    val gridHeight = HOUR * (maxOf(60, end - start) / 60f)
    // A working day that follows another day's order (a Saturday on Monday's timetable) gets a
    // column of its own, with that day's classes.
    val days = remember(classDays, dates) {
        (classDays + dates?.mapIndexedNotNull { i, d -> if (d.classes.any { it.slot.day != i }) i + 1 else null }.orEmpty()).distinct().sorted()
    }
    val todayIndex = days.indexOf(weekday)
    fun yOf(minute: Int): Dp = HOUR * ((minute - start) / 60f)
    val blocks = remember(slots, days, dates) {
        days.flatMapIndexed { col, day ->
            val classes = dates?.get(day - 1)?.classes
            val shifted = classes?.takeIf { cs -> cs.any { it.slot.day + 1 != day } }
            shifted?.map { Block(it.slot, col, it.state == ClassDay.LAB_FAT) }
                ?: slots.filter { it.day + 1 == day }.map { s -> Block(s, col, classes?.firstOrNull { it.slot == s }?.state == ClassDay.LAB_FAT) }
        }
    }
    // The class under way changes a few times a day; only then do blocks recompose.
    val liveSlot by remember(blocks, todayIndex) {
        derivedStateOf {
            val m = istMinute(now.value)
            if (todayIndex < 0) null else blocks.firstOrNull { it.column == todayIndex && it.slot.start <= m && it.slot.end > m }?.slot
        }
    }
    // What the calendar calls a class day without classes.
    val notes = if (dates == null) emptyMap() else days.associateWith { d ->
        val date = dates[d - 1]
        when (date.mark) {
            CalendarMark.HOLIDAY -> "Holiday"
            CalendarMark.NO_CLASSES -> "No class"
            CalendarMark.EXAM -> date.title
            else -> null
        }
    }.filterValues { it != null }
    val headerHeight = HEADER + if (notes.isEmpty()) 0.dp else NOTE
    val bottom = LocalContentPadding.current.calculateBottomPadding()

    // Open at the bottom: the grid runs on under the floating nav bar.
    Card(
        modifier.fillMaxSize().padding(start = 4.dp, top = 4.dp, end = 4.dp),
        shape = MaterialTheme.shapes.extraLarge.copy(bottomStart = CornerSize(0.dp), bottomEnd = CornerSize(0.dp)),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        BoxWithConstraints(Modifier.fillMaxSize().padding(end = 4.dp)) {
            val dayWidth = max(MIN_DAY, (maxWidth - GUTTER) / days.size)
            val gridWidth = dayWidth * days.size
            val viewport = maxWidth - GUTTER
            val density = LocalDensity.current
            // Open on today, centred, at the current time with an hour above it.
            val horizontal = rememberScrollState(
                with(density) {
                    if (todayIndex < 0) 0 else (dayWidth * todayIndex - (viewport - dayWidth) / 2).coerceIn(0.dp, max(0.dp, gridWidth - viewport)).roundToPx()
                },
            )
            val vertical = rememberScrollState(
                with(density) {
                    // The wall clock, not [now]: reading the state here would recompose the grid every minute.
                    val m = istMinute(System.currentTimeMillis())
                    if (todayIndex >= 0 && m in start..end) max(0.dp, yOf(m) - HOUR).roundToPx() else 0
                },
            )
            val line = scheme.outlineVariant
            Column(Modifier.fillMaxSize()) {
                // Day header: follows the grid sideways in placement only.
                Row(Modifier.fillMaxWidth().height(headerHeight)) {
                    Spacer(Modifier.width(GUTTER))
                    Box(Modifier.weight(1f).fillMaxHeight().clipToBounds()) {
                        Row(
                            Modifier
                                .wrapContentWidth(Alignment.Start, unbounded = true)
                                .width(gridWidth)
                                .fillMaxHeight()
                                .offset { IntOffset(-horizontal.value, 0) },
                        ) {
                            for (day in days) {
                                DayHeader(
                                    name = DAY_NAMES[day - 1],
                                    date = dates?.get(day - 1)?.day,
                                    isToday = day == weekday,
                                    count = blocks.count { it.column == days.indexOf(day) },
                                    // An empty note keeps every header aligned when one day has a note.
                                    note = if (notes.isEmpty()) null else notes[day] ?: "",
                                    modifier = Modifier.width(dayWidth).fillMaxHeight(),
                                )
                            }
                        }
                    }
                }
                Box(Modifier.fillMaxWidth().height(1.dp).background(line))
                Row(Modifier.fillMaxWidth().weight(1f).verticalScroll(vertical).padding(top = GRID_TOP, bottom = bottom + 12.dp)) {
                    HourLabels(start, end, h24, ::yOf, Modifier.width(GUTTER).height(gridHeight + 8.dp))
                    Box(Modifier.weight(1f).horizontalScroll(horizontal)) {
                        Layout(
                            content = {
                                for (b in blocks) key(b.column, b.slot.start) {
                                    GridBlock(b.slot, b.labFat, live = b.slot == liveSlot, height = blockHeight(b.slot, ::yOf), h24 = h24) { onOpen(b.slot) }
                                }
                            },
                            modifier = Modifier
                                .size(gridWidth, gridHeight + 8.dp)
                                .drawBehind {
                                    trace("timetable.weekly.draw") {
                                        // Today's column on the next container step, then the hour lines.
                                        if (todayIndex >= 0) {
                                            drawRoundRect(
                                                scheme.surfaceContainerHigh,
                                                Offset(dayWidth.toPx() * todayIndex, 0f),
                                                Size(dayWidth.toPx(), gridHeight.toPx()),
                                                CornerRadius(12.dp.toPx()),
                                            )
                                        }
                                        for (m in start..end step 60) drawRect(line, Offset(0f, yOf(m).toPx()), Size(size.width, 1.dp.toPx()))
                                        // The now line across the week, under the blocks. Reads the clock here only.
                                        val m = istMinute(now.value)
                                        if (todayIndex >= 0 && m in start..end) drawRect(scheme.outline, Offset(0f, yOf(m).toPx() - 0.5.dp.toPx()), Size(size.width, 1.dp.toPx()))
                                    }
                                }
                                .drawWithContent {
                                    drawContent()
                                    // Today's part of the now line, over the blocks, in primary.
                                    val m = istMinute(now.value)
                                    if (todayIndex < 0 || m !in start..end) return@drawWithContent
                                    val y = yOf(m).toPx()
                                    val x = dayWidth.toPx() * todayIndex
                                    val dot = 8.dp.toPx()
                                    drawCircle(scheme.primary, dot / 2, Offset(x + dot / 2, y))
                                    drawRoundRect(scheme.primary, Offset(x + dot / 2, y - 1.dp.toPx()), Size(dayWidth.toPx() - dot / 2, 2.dp.toPx()), CornerRadius(1.dp.toPx()))
                                },
                        ) { measurables, constraints ->
                            trace("timetable.weekly.layout") {
                                val placeables = measurables.mapIndexed { i, m ->
                                    m.measure(Constraints.fixed((dayWidth - 4.dp).roundToPx(), blockHeight(blocks[i].slot, ::yOf).roundToPx()))
                                }
                                layout(constraints.maxWidth, constraints.maxHeight) {
                                    placeables.forEachIndexed { i, p ->
                                        val b = blocks[i]
                                        p.place((dayWidth * b.column + 2.dp).roundToPx(), (yOf(b.slot.start) + 1.5.dp).roundToPx())
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

private fun blockHeight(slot: ClassSlot, yOf: (Int) -> Dp): Dp = max(30.dp, yOf(slot.end) - yOf(slot.start) - 3.dp)

/** 8, 9 … 12, 1, 2 on a 12-hour phone (the grid reads as a school day, no AM/PM noise); 8 … 13, 14 on a 24-hour one. */
@Composable
private fun HourLabels(start: Int, end: Int, h24: Boolean, yOf: (Int) -> Dp, modifier: Modifier) = Box(modifier) {
    val style = MaterialTheme.typography.labelSmall.nums()
    val color = MaterialTheme.colorScheme.onSurfaceVariant
    for (m in start..end step 60) {
        val h = if (h24) m / 60 else (m / 60 % 12).let { if (it == 0) 12 else it }
        Text("$h", Modifier.width(GUTTER - 8.dp).offset(y = yOf(m) - 7.dp), textAlign = TextAlign.End, style = style, color = color)
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun DayHeader(name: String, date: Int?, isToday: Boolean, count: Int, note: String?, modifier: Modifier) {
    val scheme = MaterialTheme.colorScheme
    Column(
        modifier.clearAndSetSemantics {
            contentDescription = "$name${date?.let { " ${dayOfMonth(it)}" }.orEmpty()}, ${if (note.isNullOrEmpty()) "$count classes" else note}"
        },
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Text(name, style = MaterialTheme.typography.labelMedium, color = if (isToday) scheme.primary else scheme.onSurfaceVariant)
        if (date != null) {
            Spacer(Modifier.height(2.dp))
            Box(Modifier.size(30.dp).then(if (isToday) Modifier.background(scheme.primary, CircleShape) else Modifier), Alignment.Center) {
                Text("${dayOfMonth(date)}", style = MaterialTheme.typography.titleSmallEmphasized.nums(), color = if (isToday) scheme.onPrimary else scheme.onSurface)
            }
        }
        if (note != null) {
            Text(note, maxLines = 1, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.labelSmall, color = AppTheme.tones.success.onSubtle)
        }
    }
}

/** One class in the grid: the kind's quiet fill (opaque, so grid lines don't show through) with an inset kind bar, outlined while live. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun GridBlock(slot: ClassSlot, labFat: Boolean, live: Boolean, height: Dp, h24: Boolean, onPress: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val isLab = slot.kind == ClassKind.LAB
    val tone = if (labFat) tones.warning else if (isLab) tones.lab else tones.accentTone
    val fill = tone.subtle
    // Lines of course name that fit under code, room and time.
    val nameLines = floor((height - 10.dp - 40.dp).value / 12f).toInt()
    fun short(m: Int) = clock(m, h24).substringBefore(' ')
    PressScale(
        onPress,
        Modifier.fillMaxSize().testTag(classTag(slot)),
        semanticsLabel = "${slot.name}, ${clock(slot.start, h24)}, room ${slot.room}",
        scale = 0.95f,
    ) {
        Column(
            Modifier
                .fillMaxSize()
                .drawBehind {
                    val r = CornerRadius(10.dp.toPx())
                    drawRoundRect(fill, cornerRadius = r)
                    drawRoundRect(tone.base, Offset(4.dp.toPx(), 5.dp.toPx()), Size(3.dp.toPx(), size.height - 10.dp.toPx()), CornerRadius(1.5.dp.toPx()))
                    if (live) {
                        val w = 1.5.dp.toPx()
                        drawRoundRect(tone.base, Offset(w / 2, w / 2), Size(size.width - w, size.height - w), CornerRadius(r.x - w / 2), Stroke(w))
                    }
                }
                .padding(start = 11.dp, top = 5.dp, end = 4.dp, bottom = 4.dp),
        ) {
            val small = MaterialTheme.typography.labelSmall.nums(fontSize = 9.5.sp)
            Text(slot.code, Modifier.fitDown(), maxLines = 1, softWrap = false, style = MaterialTheme.typography.labelMediumEmphasized.nums(fontSize = 11.sp), color = scheme.onSurface)
            Text(slot.room, maxLines = 1, overflow = TextOverflow.Clip, style = MaterialTheme.typography.labelMediumEmphasized.nums(fontSize = 10.sp), color = tone.onSubtle)
            Row(Modifier.fitDown(), verticalAlignment = Alignment.CenterVertically) {
                if (labFat) {
                    Text("LAB FAT ", style = small, color = tone.onSubtle)
                } else if (isLab) {
                    Icon(Lucide.FlaskConical, null, Modifier.size(10.dp), tint = tone.base)
                    Spacer(Modifier.width(2.dp))
                }
                Text("${short(slot.start)}–${short(slot.end)}", style = small, color = scheme.onSurfaceVariant)
            }
            if (nameLines > 0) {
                Spacer(Modifier.height(2.dp))
                Text(slot.name, maxLines = nameLines, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.labelSmall.copy(fontSize = 9.5.sp, lineHeight = 12.sp), color = scheme.onSurfaceVariant)
            }
        }
    }
}
