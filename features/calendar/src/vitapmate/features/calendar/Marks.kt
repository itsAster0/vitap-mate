package vitapmate.features.calendar

import androidx.compose.runtime.Composable
import vitapmate.core.CalendarMark
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone
import vitapmate.ui.neutralTone

/** A mark's tone on the grid, the legend and the event badges; none for a plain class day. */
@Composable
fun markTone(mark: CalendarMark): Tone? {
    val tones = AppTheme.tones
    return when (mark) {
        CalendarMark.EXAM -> tones.danger
        CalendarMark.LAB_FAT -> tones.warning
        CalendarMark.HOLIDAY -> tones.success
        CalendarMark.SPECIAL -> tones.accentTone
        CalendarMark.NO_CLASSES -> neutralTone
        CalendarMark.CLASSES -> null
    }
}

fun markLabel(mark: CalendarMark): String = when (mark) {
    CalendarMark.EXAM -> "Exam"
    CalendarMark.LAB_FAT -> "Lab FAT"
    CalendarMark.HOLIDAY -> "Holiday"
    CalendarMark.SPECIAL -> "Event"
    CalendarMark.NO_CLASSES -> "No classes"
    CalendarMark.CLASSES -> "Classes"
}

/** Days off draw a hollow dot: nothing happens, classes just don't meet. */
fun hollow(mark: CalendarMark) = mark == CalendarMark.NO_CLASSES

/** 0 = Sunday, the grid's first column (like VTOP's), for days since 1970-01-01 (a Thursday). */
fun sundayIndex(day: Int): Int = Math.floorMod(day + 4, 7)
