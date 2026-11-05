package vitapmate.features.calendar

import androidx.compose.foundation.border
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import vitapmate.core.CalendarDay
import vitapmate.core.CalendarMark
import vitapmate.core.CalendarMonth
import vitapmate.ui.AppTheme
import vitapmate.ui.Dot
import vitapmate.ui.Radii
import vitapmate.ui.formatDay
import vitapmate.ui.nums
import vitapmate.ui.tappable

val CELL_HEIGHT: Dp = 46.dp

/** Weeks [month] spans in a Sunday-first grid (4 to 6). */
fun weeksIn(month: CalendarMonth): Int = (sundayIndex(month.days.first().day) + month.days.size + 6) / 7

private val WEEKDAYS = listOf("S", "M", "T", "W", "T", "F", "S")

/** The weekday initials above the grid, Sunday first. */
@Composable
fun WeekdayHeader(modifier: Modifier = Modifier) {
    Row(modifier.fillMaxWidth()) {
        WEEKDAYS.forEach {
            Text(
                it,
                Modifier.weight(1f),
                style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold),
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
    }
}

/** One month, Sunday first like VTOP's grid: the day numbers, today ringed, a dot per marked day. */
@Composable
fun MonthGrid(month: CalendarMonth, today: Int, selected: Int?, onDay: (CalendarDay) -> Unit, modifier: Modifier = Modifier) {
    val lead = sundayIndex(month.days.first().day)
    Column(modifier.fillMaxWidth()) {
        repeat(weeksIn(month)) { week ->
            Row(Modifier.fillMaxWidth()) {
                repeat(7) { col ->
                    val day = month.days.getOrNull(week * 7 + col - lead)
                    Box(Modifier.weight(1f).height(CELL_HEIGHT), Alignment.Center) {
                        if (day != null) DayCell(day, day.day == today, day.day < today, day.day == selected, onDay)
                    }
                }
            }
        }
    }
}

@Composable
private fun DayCell(day: CalendarDay, isToday: Boolean, past: Boolean, selected: Boolean, onDay: (CalendarDay) -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val mark = day.mark
    val sunday = sundayIndex(day.day) == 0
    val off = sunday || mark == CalendarMark.HOLIDAY || mark == CalendarMark.NO_CLASSES
    // Sundays are always off; a dot there would only add noise.
    val dot = mark?.takeUnless { sunday && it == CalendarMark.HOLIDAY && day.title == "Holiday" }?.let { m -> markTone(m)?.let { m to it } }
    val shape = RoundedCornerShape(Radii.sm)
    Column(
        Modifier.testTag("calendar.day.${day.date}").tappable({ onDay(day) }, formatDay(day.day, "EEEE d MMMM") + (day.title.takeIf { it.isNotBlank() }?.let { ", $it" } ?: ""), Role.Button),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Box(
            Modifier.size(34.dp, 28.dp)
                .then(if (selected) Modifier.background(scheme.secondaryContainer, shape) else Modifier)
                .then(if (isToday) Modifier.border(1.5.dp, tones.accent, shape) else Modifier),
            Alignment.Center,
        ) {
            Text(
                "${dayOfMonth(day)}",
                style = MaterialTheme.typography.bodyMedium.nums(
                    fontWeight = if (isToday) FontWeight.Bold else FontWeight.Medium,
                    color = when {
                        selected -> scheme.onSecondaryContainer
                        isToday -> tones.accent
                        off || past -> scheme.onSurfaceVariant
                        else -> scheme.onSurface
                    },
                ),
            )
        }
        Spacer(Modifier.height(3.dp))
        if (dot != null) {
            val (m, tone) = dot
            Dot(Modifier.alpha(if (past) 0.45f else 1f), color = if (hollow(m)) Color.Transparent else tone.base, ring = if (hollow(m)) tone.base else null)
        } else {
            Spacer(Modifier.height(5.dp))
        }
    }
}

private fun dayOfMonth(day: CalendarDay) = day.date.takeLast(2).toInt()
