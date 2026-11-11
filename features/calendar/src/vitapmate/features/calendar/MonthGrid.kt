package vitapmate.features.calendar

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.Fill
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import vitapmate.core.CalendarDay
import vitapmate.core.CalendarMark
import vitapmate.core.CalendarMonth
import vitapmate.ui.AppTheme
import vitapmate.ui.formatDay
import vitapmate.ui.nums
import vitapmate.ui.tappable

/** A week row: the 40 dp band plus the gap to the next row. */
val CELL_HEIGHT: Dp = 46.dp

private val BAND_HEIGHT = 40.dp

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

/**
 * One month, Sunday first like VTOP's grid: a marked day sits on a band in its mark's tone, and
 * days in a row with the same mark join into one band; today is a primary cookie.
 */
@Composable
fun MonthGrid(month: CalendarMonth, today: Int, selected: Int?, onDay: (CalendarDay) -> Unit, modifier: Modifier = Modifier) {
    val lead = sundayIndex(month.days.first().day)
    Column(modifier.fillMaxWidth()) {
        repeat(weeksIn(month)) { week ->
            Row(Modifier.fillMaxWidth()) {
                repeat(7) { col ->
                    val at = week * 7 + col - lead
                    val day = month.days.getOrNull(at)
                    Box(Modifier.weight(1f).height(CELL_HEIGHT), Alignment.Center) {
                        if (day != null) {
                            val mark = band(day)
                            val runStart = col == 0 || band(month.days.getOrNull(at - 1)) != mark
                            val runEnd = col == 6 || band(month.days.getOrNull(at + 1)) != mark
                            DayCell(day, mark, runStart, runEnd, day.day == today, day.day < today, day.day == selected, onDay)
                        }
                    }
                }
            }
        }
    }
}

/** The mark [day] is banded in, whatever the weekday; none for a class day. */
private fun band(day: CalendarDay?): CalendarMark? = day?.mark?.takeUnless { it == CalendarMark.CLASSES }

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun DayCell(
    day: CalendarDay,
    mark: CalendarMark?,
    runStart: Boolean,
    runEnd: Boolean,
    isToday: Boolean,
    past: Boolean,
    selected: Boolean,
    onDay: (CalendarDay) -> Unit,
) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val tone = mark?.let { markTone(it) }
    val filled = mark != null && tone != null && !hollow(mark)
    val off = day.mark == CalendarMark.HOLIDAY || day.mark == CalendarMark.NO_CLASSES
    Box(
        Modifier.fillMaxWidth().height(BAND_HEIGHT)
            .testTag("calendar.day.${day.date}")
            .tappable({ onDay(day) }, formatDay(day.day, "EEEE d MMMM") + (day.title.takeIf { it.isNotBlank() }?.let { ", $it" } ?: ""), Role.Button)
            .then(
                if (tone == null) Modifier
                else Modifier.alpha(if (past) 0.55f else 1f).clipToBounds().drawBehind {
                    // One rounded rect per cell; a side that continues the run reaches past the
                    // clip, so only the run's ends show their corners.
                    val (roundLeft, roundRight) = if (layoutDirection == LayoutDirection.Ltr) runStart to runEnd else runEnd to runStart
                    val stroke = if (filled) 0f else 1.5.dp.toPx()
                    val radius = BAND_HEIGHT.toPx() / 2
                    val left = if (roundLeft) stroke / 2 else -radius * 2
                    val right = if (roundRight) size.width - stroke / 2 else size.width + radius * 2
                    drawRoundRect(
                        if (filled) tone.subtle else tone.base,
                        Offset(left, stroke / 2),
                        Size(right - left, size.height - stroke),
                        CornerRadius(radius - stroke / 2),
                        style = if (filled) Fill else Stroke(stroke),
                    )
                },
            ),
        Alignment.Center,
    ) {
        Box(
            Modifier.size(36.dp)
                .then(if (isToday) Modifier.background(scheme.primary, MaterialShapes.Cookie9Sided.toShape()) else Modifier)
                .then(if (selected) Modifier.border(2.dp, tones.accent, RoundedCornerShape(12.dp)) else Modifier),
            Alignment.Center,
        ) {
            Text(
                "${dayOfMonth(day)}",
                style = MaterialTheme.typography.bodyMedium.nums(
                    fontWeight = if (isToday) FontWeight.Bold else FontWeight.Medium,
                    color = when {
                        isToday -> scheme.onPrimary
                        filled -> tone.onSubtle
                        off || past -> scheme.onSurfaceVariant
                        else -> scheme.onSurface
                    },
                ),
            )
        }
    }
}

private fun dayOfMonth(day: CalendarDay) = day.date.takeLast(2).toInt()
