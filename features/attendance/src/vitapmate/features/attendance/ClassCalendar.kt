package vitapmate.features.attendance

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.FilledTonalIconButton
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
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronLeft
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Lucide
import java.util.Calendar
import java.util.TimeZone
import vitapmate.core.AttendedClass
import vitapmate.core.ClassStatus
import vitapmate.data.epochDayOf
import vitapmate.ui.AppTheme
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.Tone
import vitapmate.ui.formatDay
import vitapmate.ui.isoWeekday
import vitapmate.ui.nums

/**
 * A course's month calendar (`attendance.detail.calendar`), Monday first. A class day is a tile
 * filled in its worst status's tone (absent first); a day with several classes adds a pip per
 * class in that class's own tone; days without a class are faint, weekends fainter, today ringed in primary.
 * The header counts the month's classes and absences. Pages through the months with classes,
 * opening on the latest; a key to the tones sits under the grid.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ClassCalendar(classes: List<AttendedClass>) {
    // Each day's statuses, worst first.
    val byDay = remember(classes) {
        classes.groupBy { it.day }.mapValues { (_, day) -> day.map { it.status }.sortedByDescending(::rank) }
    }
    // First epoch day of each month that has classes, oldest first.
    val months = remember(byDay) { byDay.keys.map { firstOfMonth(it) }.distinct().sorted() }
    if (months.isEmpty()) return
    var index by rememberSaveable(months.size) { mutableIntStateOf(months.lastIndex) }
    val i = index.coerceIn(0, months.lastIndex)
    val scheme = MaterialTheme.colorScheme
    SectionCard(Modifier.fillMaxWidth().testTag("attendance.detail.calendar"), shape = MaterialTheme.shapes.extraLargeIncreased) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(
                    formatDay(months[i], "MMMM yyyy"),
                    Modifier.testTag("attendance.detail.calendar.month"),
                    style = MaterialTheme.typography.titleMediumEmphasized,
                )
                Tally(months[i], byDay)
            }
            Row(horizontalArrangement = Arrangement.spacedBy(2.dp)) {
                FilledTonalIconButton({ index = i - 1 }, Modifier.size(44.dp, 40.dp).testTag("attendance.detail.calendar.prev"), enabled = i > 0, shape = connected(0, 2, 20.dp, 8.dp)) {
                    Icon(Lucide.ChevronLeft, "Previous month", Modifier.size(20.dp))
                }
                FilledTonalIconButton({ index = i + 1 }, Modifier.size(44.dp, 40.dp).testTag("attendance.detail.calendar.next"), enabled = i < months.lastIndex, shape = connected(1, 2, 20.dp, 8.dp)) {
                    Icon(Lucide.ChevronRight, "Next month", Modifier.size(20.dp))
                }
            }
        }
        Row(Modifier.fillMaxWidth().padding(top = Space.md, bottom = Space.xs)) {
            listOf("M", "T", "W", "T", "F", "S", "S").forEachIndexed { col, d ->
                Text(
                    d,
                    Modifier.weight(1f),
                    style = MaterialTheme.typography.labelMedium,
                    color = scheme.onSurfaceVariant.copy(alpha = if (col >= 5) WEEKEND else 1f),
                    textAlign = TextAlign.Center,
                )
            }
        }
        val spatial = MaterialTheme.motionScheme.defaultSpatialSpec<IntOffset>()
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
        // The key to the tiles' tones.
        FlowRow(Modifier.padding(top = Space.md), horizontalArrangement = Arrangement.spacedBy(10.dp), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
            for (status in listOf(ClassStatus.PRESENT, ClassStatus.ON_DUTY, ClassStatus.ABSENT, null)) {
                val (label, tone) = status.look()
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(Modifier.size(10.dp).background(tone.base, CircleShape))
                    Text(label, Modifier.padding(start = 5.dp), style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
                }
            }
        }
    }
}

/** "12 classes · 2 absent" for the month starting on [first]; the absences in the danger tone. */
@Composable
private fun Tally(first: Int, byDay: Map<Int, List<ClassStatus?>>) {
    val inMonth = byDay.filterKeys { firstOfMonth(it) == first }.values.flatten()
    val absent = inMonth.count { it == ClassStatus.ABSENT }
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    val danger = AppTheme.tones.danger.base
    Text(
        buildAnnotatedString {
            withStyle(SpanStyle(fontWeight = FontWeight.SemiBold)) { append("${inMonth.size}") }
            append(if (inMonth.size == 1) " class" else " classes")
            if (absent > 0) {
                append(" · ")
                withStyle(SpanStyle(color = danger, fontWeight = FontWeight.SemiBold)) { append("$absent absent") }
            }
        },
        Modifier.testTag("attendance.detail.calendar.tally"),
        style = MaterialTheme.typography.bodySmall.nums(),
        color = muted,
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Month(first: Int, byDay: Map<Int, List<ClassStatus?>>) {
    val scheme = MaterialTheme.colorScheme
    val leading = isoWeekday(first) - 1
    val length = daysInMonth(first)
    val today = remember { epochDayOf(System.currentTimeMillis()) }
    val tile = CircleShape
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        for (week in 0 until (leading + length + 6) / 7) {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                for (col in 0 until 7) {
                    val n = week * 7 + col - leading
                    val day = first + n
                    val inMonth = n in 0 until length
                    val statuses = if (inMonth) byDay[day] else null
                    val tone: Tone? = statuses?.first()?.look()?.second
                    val pips = statuses?.takeIf { it.size > 1 }
                    Box(
                        Modifier.weight(1f).height(42.dp)
                            .then(if (inMonth) Modifier.testTag("attendance.detail.calendar.$day") else Modifier)
                            .then(if (tone != null) Modifier.background(tone.subtle, tile) else Modifier)
                            .then(if (inMonth && day == today) Modifier.border(2.dp, scheme.primary, tile) else Modifier),
                        Alignment.Center,
                    ) {
                        if (!inMonth) return@Box
                        Text(
                            "${n + 1}",
                            Modifier.padding(bottom = if (pips != null) 6.dp else 0.dp),
                            style = MaterialTheme.typography.labelLarge.nums(),
                            fontWeight = when {
                                tone != null -> FontWeight.ExtraBold
                                day == today -> FontWeight.Bold
                                else -> FontWeight.Normal
                            },
                            color = when {
                                tone != null -> tone.onSubtle
                                day == today -> scheme.primary
                                else -> scheme.onSurfaceVariant.copy(alpha = if (col >= 5) WEEKEND else IDLE)
                            },
                        )
                        if (pips != null) {
                            Row(
                                Modifier.align(Alignment.BottomCenter).padding(bottom = 7.dp),
                                horizontalArrangement = Arrangement.spacedBy(3.dp),
                            ) {
                                for (s in pips.take(4)) {
                                    Box(Modifier.size(width = 6.dp, height = 3.dp).background(s.look().second.base, CircleShape))
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/** How faint a day without a class is, and a weekend (also the weekend column headers). */
private const val IDLE = 0.75f
private const val WEEKEND = 0.45f

/** Which status a day with several classes shows: absent, on duty, present, marked, not posted. */
private fun rank(s: ClassStatus?) = when (s) {
    ClassStatus.ABSENT -> 4
    ClassStatus.ON_DUTY -> 3
    ClassStatus.PRESENT -> 2
    ClassStatus.OTHER -> 1
    null -> 0
}

internal fun firstOfMonth(day: Int) = day - formatDay(day, "d").toInt() + 1

internal fun daysInMonth(first: Int): Int =
    Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply { timeInMillis = first * 86_400_000L }.getActualMaximum(Calendar.DAY_OF_MONTH)
