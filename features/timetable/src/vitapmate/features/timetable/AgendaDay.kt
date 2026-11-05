package vitapmate.features.timetable

import vitapmate.core.AttendanceCourse
import vitapmate.core.CalendarMark
import vitapmate.core.ClassDay
import vitapmate.core.ClassSlot
import vitapmate.core.DayClass
import vitapmate.core.TimetableDay

/** VIT leaves 10 minutes between classes; only longer gaps are free time. */
const val MIN_BREAK = 20

enum class ClassState { Done, Now, Next, Later }

/** The classes of [day] that meet (held, or its Lab FAT). */
fun meeting(day: TimetableDay): List<DayClass> = day.classes.filter { it.state != ClassDay.OFF }

/**
 * The attendance course for a class (same code, lab matched to lab); null when attendance has
 * no such course or VTOP has not counted a class yet, so no "0%" shows for a new course.
 */
fun List<AttendanceCourse>.forSlot(slot: ClassSlot): AttendanceCourse? =
    firstOrNull { it.code == slot.code && it.kind == slot.kind && it.total > 0 }

/** Test tag of a class, in the agenda and the grid: `timetable.class.<day 0=Mon>.<start minute>`. */
fun classTag(slot: ClassSlot) = "timetable.class.${slot.day}.${slot.start}"

/** What the calendar calls a day without your classes. */
class DayOff(val mark: CalendarMark, val title: String)

/**
 * One day of the agenda at [minuteNow] on [today] (India time): the regular [schedule] is always
 * listed; [slots] are the classes that meet, which drive the focus card, day bar and statuses.
 */
class AgendaDay(val day: TimetableDay, today: Int, val minuteNow: Int) {
    val schedule = day.classes
    val slots = meeting(day)
    val labFats = slots.filter { it.state == ClassDay.LAB_FAT }.toSet()
    val isToday = day.day == today
    val past = day.day < today
    val current = if (isToday) slots.firstOrNull { it.slot.start <= minuteNow && it.slot.end > minuteNow } else null
    val next = if (past) null else slots.firstOrNull { !isToday || it.slot.start > minuteNow }
    val remaining = if (isToday) slots.count { it.slot.end > minuteNow } else if (past) 0 else slots.size

    /** Today before any class has started. */
    val beforeFirst = isToday && slots.isNotEmpty() && minuteNow < slots.first().slot.start

    /** Today after the last class, or an earlier day: the card recaps it. */
    val done = slots.isNotEmpty() && current == null && next == null

    /** Classes whose times overlap another class that day (VTOP allows a clash in rare cases). */
    val clashes: Set<DayClass> = slots.filterIndexed { i, a ->
        slots.withIndex().any { (j, b) -> i != j && a.slot.start < b.slot.end && b.slot.start < a.slot.end }
    }.toSet()

    // A class day where only labs are off (after the Lab FAT) is no day off.
    val dayOff = day.mark?.takeIf { slots.isEmpty() && schedule.isNotEmpty() && !day.holdsTheory }?.let { DayOff(it, day.title.orEmpty()) }

    /** What the calendar calls the day when it has none of your classes (holiday, exam, no classes). */
    val freeReason: DayOff? = dayOff ?: day.mark?.takeIf {
        slots.isEmpty() && it != CalendarMark.CLASSES && it != CalendarMark.SPECIAL && it != CalendarMark.LAB_FAT
    }?.let { DayOff(it, day.title.orEmpty()) }

    // Plain "Holiday" Sundays and "No instructional day" Mondays say nothing new, so only named
    // days get a note. Lab FAT only when you write one.
    val note: String? = when {
        slots.isNotEmpty() && day.lastClassDay -> "Last class day"
        dayOff != null -> dayOff.title
        day.mark == null || day.mark == CalendarMark.CLASSES ||
            (day.mark == CalendarMark.LAB_FAT && labFats.isEmpty()) ||
            day.title in setOf("Holiday", "No instructional day") -> null
        else -> day.title
    }

    // Free time between classes; the longest is tagged when the day has more than one.
    private val gaps = schedule.zipWithNext { a, b -> b.slot.start - a.slot.end }
    private val longestGap = gaps.filter { it >= MIN_BREAK }.takeIf { it.size > 1 }?.max()

    /** Free minutes before schedule[i] (0 for the first, negative for a clash). */
    fun gapBefore(i: Int): Int = if (i == 0) 0 else gaps[i - 1]

    fun isLongestGap(i: Int) = i > 0 && gaps[i - 1] == longestGap

    fun isLastClass(c: DayClass) = c.state == ClassDay.HELD && c.lastClass

    fun stateOf(c: DayClass): ClassState {
        val live = isToday && c.state != ClassDay.OFF
        return when {
            c == current -> ClassState.Now
            c == next && live -> ClassState.Next
            past || (live && c.slot.end <= minuteNow) -> ClassState.Done
            else -> ClassState.Later
        }
    }
}
