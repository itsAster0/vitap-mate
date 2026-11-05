package vitapmate.features.attendance

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronLeft
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Lucide
import java.util.Calendar
import java.util.TimeZone
import vitapmate.core.AttendedClass
import vitapmate.core.ClassStatus
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.Tone
import vitapmate.ui.formatDay
import vitapmate.ui.isoWeekday
import vitapmate.ui.nums

/**
 * The Flutter app's month calendar of a course (`attendance.detail.calendar`): Monday first, each
 * class day filled in its status's tone (several classes on a day: the worst, absent first), today
 * ringed. Pages through the months with classes, opening on the latest.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ClassCalendar(classes: List<AttendedClass>) {
    val byDay = remember(classes) {
        classes.groupBy { it.day }.mapValues { (_, day) -> day.maxBy { rank(it.status) }.status }
    }
    // First epoch day of each month that has classes, oldest first.
    val months = remember(byDay) { byDay.keys.map { firstOfMonth(it) }.distinct().sorted() }
    if (months.isEmpty()) return
    var index by rememberSaveable(months.size) { mutableIntStateOf(months.lastIndex) }
    val i = index.coerceIn(0, months.lastIndex)
    val scheme = MaterialTheme.colorScheme
    SectionCard(Modifier.fillMaxWidth().testTag("attendance.detail.calendar")) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                formatDay(months[i], "MMMM yyyy"),
                Modifier.weight(1f).testTag("attendance.detail.calendar.month"),
                style = MaterialTheme.typography.titleMediumEmphasized,
            )
            IconButton({ index = i - 1 }, modifier = Modifier.testTag("attendance.detail.calendar.prev"), enabled = i > 0, shapes = IconButtonDefaults.shapes()) {
                Icon(Lucide.ChevronLeft, "Previous month", Modifier.size(20.dp))
            }
            IconButton({ index = i + 1 }, modifier = Modifier.testTag("attendance.detail.calendar.next"), enabled = i < months.lastIndex, shapes = IconButtonDefaults.shapes()) {
                Icon(Lucide.ChevronRight, "Next month", Modifier.size(20.dp))
            }
        }
        Row(Modifier.fillMaxWidth().padding(top = Space.xs, bottom = Space.xs)) {
            for (d in listOf("M", "T", "W", "T", "F", "S", "S")) {
                Text(d, Modifier.weight(1f), style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant, textAlign = androidx.compose.ui.text.style.TextAlign.Center)
            }
        }
        val spatial = MaterialTheme.motionScheme.defaultSpatialSpec<androidx.compose.ui.unit.IntOffset>()
        val effects = MaterialTheme.motionScheme.defaultEffectsSpec<Float>()
        AnimatedContent(
            months[i],
            transitionSpec = {
                val dir = if (targetState > initialState) 1 else -1
                (slideInHorizontally(spatial) { it / 4 * dir } + fadeIn(effects)) togetherWith (slideOutHorizontally(spatial) { -it / 4 * dir } + fadeOut(effects))
            },
            label = "calendar-month",
        ) { first ->
            Month(first, byDay)
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Month(first: Int, byDay: Map<Int, ClassStatus?>) {
    val scheme = MaterialTheme.colorScheme
    val leading = isoWeekday(first) - 1
    val length = daysInMonth(first)
    val today = remember { System.currentTimeMillis().let { Math.floorDiv(it + TimeZone.getDefault().getOffset(it), 86_400_000L).toInt() } }
    Column {
        for (week in 0 until (leading + length + 6) / 7) {
            Row(Modifier.fillMaxWidth()) {
                for (col in 0 until 7) {
                    val n = week * 7 + col - leading
                    Box(Modifier.weight(1f).height(40.dp), Alignment.Center) {
                        if (n in 0 until length) {
                            val day = first + n
                            val tone: Tone? = if (day in byDay) byDay[day].look().second else null
                            Box(
                                Modifier.size(34.dp)
                                    .then(if (tone != null) Modifier.background(tone.subtle, CircleShape).border(1.5.dp, tone.base, CircleShape) else Modifier)
                                    .then(if (tone == null && day == today) Modifier.border(1.dp, scheme.outline, CircleShape) else Modifier)
                                    .testTag("attendance.detail.calendar.$day"),
                                Alignment.Center,
                            ) {
                                Text(
                                    "${n + 1}",
                                    style = MaterialTheme.typography.labelLarge.nums(),
                                    fontWeight = if (tone != null) FontWeight.Bold else FontWeight.Normal,
                                    color = tone?.onSubtle ?: scheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

/** Which status a day with several classes shows: absent, on duty, present, marked, not posted. */
private fun rank(s: ClassStatus?) = when (s) {
    ClassStatus.ABSENT -> 4
    ClassStatus.ON_DUTY -> 3
    ClassStatus.PRESENT -> 2
    ClassStatus.OTHER -> 1
    null -> 0
}

private fun firstOfMonth(day: Int) = day - formatDay(day, "d").toInt() + 1

private fun daysInMonth(first: Int): Int =
    Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply { timeInMillis = first * 86_400_000L }.getActualMaximum(Calendar.DAY_OF_MONTH)
