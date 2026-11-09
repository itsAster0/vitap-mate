package vitapmate.features.attendance

import androidx.compose.foundation.background
import vitapmate.ui.isoWeekday
import com.composables.icons.lucide.ChevronUp
import com.composables.icons.lucide.CalendarDays
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.draw.clip
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.clickable
import androidx.compose.foundation.border
import androidx.compose.animation.animateContentSize
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Info
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.X
import kotlinx.coroutines.launch
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendancePlan
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.OpenHaptic
import vitapmate.ui.Space
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/** What a tap on a day marks it as. */
private enum class Mark { Skip, Attend }

/**
 * "Plan days": the days still to come before each course's next exam (from [outlook]), the next 7
 * as chips or all of them on a month calendar. With Skip or Attend chosen, tapping a day marks it
 * so (again: clears it). Every course with classes
 * on the marked days, with what it attends and misses there, and the percentage VTOP would show
 * after them, like the calculator's What if (unmarked days don't count). Nine tenths of the screen.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
internal fun SkipDaySheet(courses: List<AttendanceCourse>, outlook: Map<String, AttendancePlan>, onDismiss: () -> Unit) {
    val scope = rememberCoroutineScope()
    val tones = AppTheme.tones
    val days = outlook.values.flatMap { plan -> plan.days.map { it.day } }.distinct().sorted()
    var mark by rememberSaveable { mutableStateOf(Mark.Skip) }
    var skipped by rememberSaveable { mutableStateOf(emptyList<Int>()) }
    var attended by rememberSaveable { mutableStateOf(emptyList<Int>()) }
    var calendar by rememberSaveable { mutableStateOf(false) }
    val log: (String, Map<String, String>) -> Unit = { what, fields ->
        scope.launch { flow("attendance.plan_days") { id -> Log.info("ui.attendance", "plan days $what", id, fields) } }
    }
    OpenHaptic()
    ModalBottomSheet(
        onDismiss,
        Modifier.semantics { testTagsAsResourceId = true }.testTag("attendance.skip_day"),
        rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded)),
        containerColor = MaterialTheme.colorScheme.surface,
        dragHandle = {
            Box(Modifier.fillMaxWidth().padding(top = Space.sm, bottom = Space.xs), Alignment.Center) {
                Box(Modifier.size(32.dp, 4.dp).background(MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.4f), CircleShape))
            }
        },
    ) {
        Column(Modifier.fillMaxWidth().fillMaxHeight(0.9f)) {
            Row(Modifier.padding(start = Space.lg, end = Space.sm, top = Space.sm), verticalAlignment = Alignment.CenterVertically) {
                Text("Plan days", Modifier.weight(1f), style = MaterialTheme.typography.titleLargeEmphasized)
                TextButton(
                    { skipped = emptyList(); attended = emptyList(); log("clear", emptyMap()) },
                    Modifier.testTag("attendance.skip_day.clear"),
                    enabled = skipped.isNotEmpty() || attended.isNotEmpty(),
                ) { Text("Clear") }
            }
            Text(
                "Mark the days you'll skip or attend: where each course would stand after them",
                Modifier.padding(start = Space.lg, end = Space.lg, top = Space.xs, bottom = Space.md),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            if (days.isEmpty()) {
                Text("No classes left before the next exam.", Modifier.padding(Space.lg), style = MaterialTheme.typography.bodyLarge)
                return@Column
            }
            val stateOf = { d: Int ->
                when (d) {
                    in skipped -> Mark.Skip
                    in attended -> Mark.Attend
                    else -> null
                }
            }
            // Same mark again clears the day; the other mark moves it over.
            val toggle = { d: Int ->
                val state = stateOf(d)
                skipped = skipped - d
                attended = attended - d
                if (state != mark) {
                    if (mark == Mark.Skip) skipped = skipped + d else attended = attended + d
                }
                log("day", mapOf("day" to formatDay(d, "yyyy-MM-dd"), "mark" to if (state == mark) "none" else mark.name.lowercase()))
            }
            val hit = courses.mapNotNull { course ->
                val plan = outlook[course.key] ?: return@mapNotNull null
                val skip = plan.days.filter { it.day in skipped }.sumOf { it.classes }
                val attend = plan.days.filter { it.day in attended }.sumOf { it.classes }
                if (skip + attend == 0) null else Triple(course, attend, skip)
            }.sortedBy { (course, attend, skip) -> course.projected(attend, skip).first }
            LazyColumn(
                Modifier.fillMaxWidth().weight(1f),
                contentPadding = PaddingValues(bottom = Space.xl),
                verticalArrangement = Arrangement.spacedBy(Space.md),
            ) {
                item("mark") {
                    ConnectedChoice(
                        Mark.entries,
                        mark,
                        { m -> mark = m; log("mark", mapOf("mark" to m.name.lowercase())) },
                        tag = "attendance.skip_day.mark",
                        key = { it.name.lowercase() },
                        label = { it.name },
                        icon = { if (it == Mark.Skip) Lucide.X else Lucide.Check },
                        modifier = Modifier.padding(horizontal = Space.lg),
                    )
                }
                item("days") {
                    Column(Modifier.animateContentSize(MaterialTheme.motionScheme.defaultSpatialSpec())) {
                        Row(Modifier.padding(start = Space.lg, end = Space.sm), verticalAlignment = Alignment.CenterVertically) {
                            Text(if (calendar) "Every class day" else "Next 7 days", Modifier.weight(1f), style = MaterialTheme.typography.titleSmall)
                            TextButton(
                                { calendar = !calendar; log("calendar", mapOf("open" to "$calendar")) },
                                Modifier.testTag("attendance.skip_day.calendar"),
                            ) {
                                Icon(if (calendar) Lucide.ChevronUp else Lucide.CalendarDays, null, Modifier.size(16.dp))
                                Text(if (calendar) "  Week" else "  Calendar")
                            }
                        }
                        if (calendar) {
                            PlanCalendar(days, stateOf, toggle, Modifier.padding(horizontal = Space.lg))
                        } else {
                            val week = days.filter { it < days.first() + 7 }
                            LazyRow(horizontalArrangement = Arrangement.spacedBy(Space.sm), contentPadding = PaddingValues(horizontal = Space.lg)) {
                                items(week, key = { it }) { d ->
                                    val state = stateOf(d)
                                    val tone = if (state == Mark.Skip) tones.danger else tones.success
                                    FilterChip(
                                        state != null,
                                        { toggle(d) },
                                        { Text(formatDay(d, "EEE d MMM"), style = MaterialTheme.typography.labelLarge.nums()) },
                                        Modifier.testTag("attendance.skip_day.$d"),
                                        leadingIcon = state?.let { st -> { Icon(if (st == Mark.Skip) Lucide.X else Lucide.Check, null, Modifier.size(16.dp)) } },
                                        colors = FilterChipDefaults.filterChipColors(
                                            selectedContainerColor = tone.subtle,
                                            selectedLabelColor = tone.onSubtle,
                                            selectedLeadingIconColor = tone.base,
                                        ),
                                    )
                                }
                            }
                        }
                    }
                }
                if (skipped.isEmpty() && attended.isEmpty()) {
                    item("hint") {
                        Text("Tap a day above.", Modifier.padding(horizontal = Space.lg), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    return@LazyColumn
                }
                // Held but not on VTOP yet: not counted, so the numbers can still move.
                val behind = hit.count { (course, _, _) -> (outlook[course.key]?.unposted ?: 0) > 0 }
                if (behind > 0) item("unposted") { UnpostedNote(behind, Modifier.padding(horizontal = Space.lg)) }
                // Lowest first: the course in most danger leads.
                items(hit, key = { it.first.key }) { (course, attend, skip) ->
                    PlanRow(course, attend, skip, outlook[course.key]?.unposted ?: 0, Modifier.padding(horizontal = Space.lg))
                }
                val others = outlook.size - hit.size
                if (others > 0) item("others") {
                    Text(
                        "$others other ${if (others == 1) "course has" else "courses have"} no class on those days",
                        Modifier.padding(horizontal = Space.lg),
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}

/**
 * Every month holding [days], Monday first. A class day is a tile: outlined until marked, then
 * filled red (skip) or green (attend) with its icon; tapping it calls [onTap]. Other days are
 * faint and can't be tapped.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun PlanCalendar(days: List<Int>, stateOf: (Int) -> Mark?, onTap: (Int) -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val open = days.toSet()
    val tile = RoundedCornerShape(12.dp)
    Column(modifier.testTag("attendance.skip_day.month"), verticalArrangement = Arrangement.spacedBy(Space.md)) {
        for (first in days.map { firstOfMonth(it) }.distinct()) {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(formatDay(first, "MMMM"), style = MaterialTheme.typography.labelLarge, color = scheme.onSurfaceVariant)
                Row(Modifier.fillMaxWidth()) {
                    for (d in listOf("M", "T", "W", "T", "F", "S", "S")) {
                        Text(d, Modifier.weight(1f), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant, textAlign = TextAlign.Center)
                    }
                }
                val leading = isoWeekday(first) - 1
                val length = daysInMonth(first)
                for (week in 0 until (leading + length + 6) / 7) {
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                        for (col in 0 until 7) {
                            val n = week * 7 + col - leading
                            val day = first + n
                            val usable = n in 0 until length && day in open
                            val state = if (usable) stateOf(day) else null
                            val tone = if (state == Mark.Skip) tones.danger else tones.success
                            Box(
                                Modifier.weight(1f).height(44.dp)
                                    .then(
                                        when {
                                            state != null -> Modifier.background(tone.subtle, tile).border(1.5.dp, tone.base, tile)
                                            usable -> Modifier.border(1.dp, scheme.outlineVariant, tile)
                                            else -> Modifier
                                        },
                                    )
                                    .then(if (usable) Modifier.clip(tile).clickable { onTap(day) }.testTag("attendance.skip_day.$day") else Modifier),
                                Alignment.Center,
                            ) {
                                if (n !in 0 until length) return@Box
                                if (state != null) {
                                    Icon(if (state == Mark.Skip) Lucide.X else Lucide.Check, null, Modifier.size(12.dp).align(Alignment.TopEnd).padding(top = 4.dp, end = 4.dp), tint = tone.base)
                                }
                                Text(
                                    "${n + 1}",
                                    style = MaterialTheme.typography.labelLarge.nums(),
                                    fontWeight = if (usable) FontWeight.Bold else FontWeight.Normal,
                                    color = when {
                                        state != null -> tone.onSubtle
                                        usable -> scheme.onSurface
                                        else -> scheme.onSurfaceVariant.copy(alpha = 0.4f)
                                    },
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

/**
 * One line in the warning tone: [courses] have classes held but not on VTOP yet, which these
 * numbers leave out until they are posted.
 */
@Composable
private fun UnpostedNote(courses: Int, modifier: Modifier = Modifier) {
    val tone = AppTheme.tones.warning
    Row(
        modifier.fillMaxWidth().background(tone.subtle, RoundedCornerShape(12.dp)).padding(Space.md).testTag("attendance.skip_day.unposted"),
        verticalAlignment = Alignment.Top,
    ) {
        Icon(Lucide.Info, null, Modifier.size(16.dp).padding(top = 2.dp), tint = tone.base)
        Text(
            "${if (courses == 1) "1 course has" else "$courses courses have"} classes VTOP hasn't posted yet. " +
                "They aren't counted here, so these numbers can change once they're posted.",
            Modifier.padding(start = Space.sm),
            style = MaterialTheme.typography.bodySmall,
            color = tone.onSubtle,
        )
    }
}

/**
 * A course on the marked days: what it attends and misses (and any classes not posted yet), then
 * "92% → 89%" as VTOP would show it.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun PlanRow(course: AttendanceCourse, attend: Int, skip: Int, unposted: Int, modifier: Modifier = Modifier) {
    val tones = AppTheme.tones
    val (percent, exactSafe) = course.projected(attend, skip)
    val tone = when {
        percent < 75 -> tones.danger
        exactSafe -> tones.success
        else -> tones.warning
    }
    val unit = { n: Int -> course.labs(n).ifEmpty { if (n == 1) " class" else " classes" } }
    Row(modifier.fillMaxWidth().testTag("attendance.skip_day.course.${course.key}"), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text(course.name.ifBlank { course.code }, style = MaterialTheme.typography.titleSmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text(
                listOfNotNull(
                    if (course.isLab) "Lab" else "Theory",
                    "attends $attend${unit(attend)}".takeIf { attend > 0 },
                    "misses $skip${unit(skip)}".takeIf { skip > 0 },
                    "$unposted not posted".takeIf { unposted > 0 },
                ).joinToString(" · "),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Text("${course.projected(0, 0).first}% → ", style = MaterialTheme.typography.labelLarge.nums(), color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(
            "$percent%",
            Modifier.background(tone.subtle, RoundedCornerShape(8.dp)).padding(horizontal = 8.dp, vertical = 2.dp),
            style = MaterialTheme.typography.labelLargeEmphasized.nums(),
            color = tone.onSubtle,
        )
    }
}
