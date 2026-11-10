package vitapmate.features.timetable

import androidx.compose.runtime.getValue

import androidx.compose.animation.core.spring
import androidx.compose.animation.core.Spring
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.snapshotFlow
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CornerSize
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.State
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.core.AttendanceCourse
import vitapmate.core.ClassDay
import vitapmate.core.ClassKind
import vitapmate.core.DayClass
import vitapmate.core.TimetableDay
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.ScrollToTopOnReselect
import vitapmate.ui.clock
import vitapmate.ui.dayLabel
import vitapmate.ui.formatDay
import vitapmate.ui.isoWeekday

/**
 * The day view: the week strip, the selected day's heading and shape, its focus card (now / next,
 * the end-of-day summary, or why it's free), then its classes with the free time between them; a
 * day your timetable has nothing on looks at the week ahead and the courses to watch. Days are
 * pages: swipe sideways for the next or previous day. [days] start on this week's Monday (India time).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun AgendaView(
    days: List<TimetableDay>,
    attendance: List<AttendanceCourse>,
    selectedDay: Int,
    onSelectDay: (day: Int, how: String) -> Unit,
    onOpen: (DayClass, from: String) -> Unit,
    now: State<Long>,
    h24: Boolean,
    modifier: Modifier = Modifier,
) {
    val ms = now.value // statuses change on minute boundaries
    val today = istDay(ms)
    val minuteNow = istMinute(ms)
    val first = days.first().day
    val week = days.subList(0, 7)
    fun at(d: Int) = days.getOrNull(d - first)

    // One page per weekday: swiping follows the finger and settles on a spring; the strip and
    // the pager move together.
    val pager = rememberPagerState(initialPage = selectedDay - 1) { 7 }
    // The collector outlives recompositions: read the latest day and callback, never the first ones
    // (the screen's state is recreated once the semester is known, so a captured callback goes stale).
    val latest by rememberUpdatedState(selectedDay)
    val select by rememberUpdatedState(onSelectDay)
    LaunchedEffect(selectedDay) {
        if (pager.targetPage != selectedDay - 1) pager.animateScrollToPage(selectedDay - 1, animationSpec = MotionSpring)
    }
    LaunchedEffect(pager) {
        snapshotFlow { pager.settledPage }.collect { p -> if (p + 1 != latest) select(p + 1, "swipe") }
    }
    Column(modifier.fillMaxSize()) {
        WeekStrip(
            dates = week.map { it.day },
            today = if (today - first in 0..6) isoWeekday(today) else 0,
            selectedDay = selectedDay,
            onSelect = { onSelectDay(it, "tap") },
            classCounts = week.map { meeting(it).size },
            examDays = week.mapIndexedNotNull { i, d -> if (d.papers.isNotEmpty()) i + 1 else null }.toSet(),
            modifier = Modifier.padding(horizontal = 8.dp),
        )
        Spacer(Modifier.height(12.dp))
        HorizontalPager(pager, Modifier.fillMaxWidth().weight(1f), pageSpacing = 16.dp, beyondViewportPageCount = 1, key = { it }) { page ->
            val weekday = page + 1
            val scroll = rememberScrollState()
            if (page == pager.currentPage) ScrollToTopOnReselect(scroll)
            val d = AgendaDay(week[page], today, minuteNow)
            Column(
                Modifier
                    .fillMaxSize()
                    .verticalScroll(scroll)
                    .padding(horizontal = 8.dp)
                    .padding(bottom = LocalContentPadding.current.calculateBottomPadding() + 16.dp)
                    .testTag("timetable.agenda.day.$weekday"),
            ) {
                Column(Modifier.padding(horizontal = 4.dp)) {
                    DayHeading(d.day.day, d.isToday, d.slots, d.note, onToday = { onSelectDay(isoWeekday(today), "today") }, h24 = h24)
                    if (d.slots.isNotEmpty()) {
                        Spacer(Modifier.height(12.dp))
                        DayBar(d.slots.map { it.slot }, minuteNow = if (d.isToday) minuteNow else null)
                    }
                }
                Spacer(Modifier.height(16.dp))
                DayContent(d, ::at, attendance, today, h24, onOpen = { onOpen(it, "agenda") }, onGo = { date ->
                    if (date - first in 0..6) ({ onSelectDay(isoWeekday(date), "next_class_day") }) else null
                })
                zoneNote(ms)?.let {
                    Spacer(Modifier.height(20.dp))
                    Text(
                        it,
                        Modifier.fillMaxWidth().padding(horizontal = 8.dp).testTag("timetable.zone"),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                UpdatedFooter(Modifier.padding(top = 8.dp))
            }
        }
    }
}

private val Outer = CornerSize(22.dp)
private val Inner = CornerSize(6.dp)

/** The settle spring for tapping a day: a little bounce, quick to rest. */
private val MotionSpring = spring<Float>(dampingRatio = 0.8f, stiffness = Spring.StiffnessMediumLow)

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun DayContent(
    d: AgendaDay,
    days: (Int) -> TimetableDay?,
    attendance: List<AttendanceCourse>,
    today: Int,
    h24: Boolean,
    onOpen: (DayClass) -> Unit,
    onGo: (Int) -> (() -> Unit)?,
) = Column(Modifier.fillMaxWidth()) {
    val date = d.day.day
    val minuteNow = d.minuteNow
    when {
        d.done -> DaySummary(d.slots, d.labFats, d.isToday)
        d.current != null || d.next != null -> FocusCard(d, today, attendance, h24, onOpen)
        else -> FreeDayCard(
            d,
            countdown = listOfNotNull(
                d.day.classDaysLeft.takeIf { it > 0 }?.let { "$it class ${if (it == 1) "day" else "days"} left" },
                d.day.upcomingExam?.let { "${it.name} starts ${formatDay(it.day, "EEE d MMM")}" },
            ).joinToString(" · ").ifEmpty { null },
            h24 = h24,
        )
    }

    // A day your timetable has no classes on looks at the week ahead and the courses to watch.
    if (d.schedule.isEmpty()) {
        WeekAhead(from = date, days = days, onOpen = onGo, h24 = h24)
        AttendanceWatch(attendance)
        return@Column
    }
    SectionTitle(
        if (d.isToday) "Classes" else "Schedule",
        when {
            d.slots.isEmpty() -> "No classes"
            d.isToday && d.remaining in 1 until d.slots.size -> "${d.remaining} of ${d.slots.size} left"
            else -> "${d.slots.size} ${if (d.slots.size == 1) "class" else "classes"}"
        },
    )
    // Back-to-back classes form one group; free time splits groups.
    val groups = mutableListOf<MutableList<Int>>()
    d.schedule.indices.forEach { i -> if (i == 0 || d.gapBefore(i) >= MIN_BREAK) groups += mutableListOf(i) else groups.last() += i }
    groups.forEach { group ->
        val i0 = group.first()
        if (i0 > 0) {
            val prev = d.schedule[i0 - 1].slot
            val next = d.schedule[i0].slot
            GapRow(
                prev.end,
                next.start,
                isNow = d.isToday && d.dayOff == null && prev.end <= minuteNow && next.start > minuteNow,
                done = d.stateOf(d.schedule[i0]).let { it == ClassState.Done || it == ClassState.Now },
                minuteNow,
                h24,
            )
        }
        Column(verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
            group.forEachIndexed { k, i ->
                val c = d.schedule[i]
                // Not meeting (a holiday, or a lab after the Lab FAT): listed, with no live state.
                val off = c.state == ClassDay.OFF
                val labFat = c.state == ClassDay.LAB_FAT
                ClassItem(
                    slot = c.slot,
                    state = d.stateOf(c),
                    // Large outer corners on the group, small inner ones between its classes.
                    shapes = ListItemDefaults.shapes(
                        shape = RoundedCornerShape(
                            if (k == 0) Outer else Inner,
                            if (k == 0) Outer else Inner,
                            if (k == group.size - 1) Outer else Inner,
                            if (k == group.size - 1) Outer else Inner,
                        ),
                    ),
                    // A Lab FAT doesn't count towards attendance.
                    record = if (labFat) null else attendance.forSlot(c.slot),
                    minuteNow = if (d.isToday && !off) minuteNow else null,
                    labFat = labFat,
                    lastClass = d.isLastClass(c),
                    clash = c in d.clashes,
                    // On a class day, a lab that doesn't meet says so; a day off says it once, in the card.
                    note = if (off && d.dayOff == null && c.slot.kind == ClassKind.LAB) "No lab" else null,
                    showPosted = d.done && !off && !labFat,
                    posted = c.posted,
                    first = i == 0,
                    last = i == d.schedule.lastIndex,
                    h24 = h24,
                    onOpen = { onOpen(c) },
                )
            }
        }
    }
}
