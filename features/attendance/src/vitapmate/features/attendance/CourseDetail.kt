package vitapmate.features.attendance

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.material3.Icon
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.LoadingIndicator
import androidx.compose.ui.text.style.TextOverflow
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.RefreshCw
import vitapmate.data.Histories
import vitapmate.data.PageState
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.SecondaryTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendanceDetail
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PageScaffold
import vitapmate.ui.KindChip
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.StatTile
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.nums
import vitapmate.ui.rememberSyncedPager

/** The course sheet's tabs. */
private enum class DetailTab(val label: String) { Overview("Overview"), Calculator("Calculator") }

/**
 * One course, inside [CourseSheet] ([key] is [AttendanceCourse.key], enough to reload after process death),
 * `page.attendance_detail` (the saved history first, then VTOP), in two tabs over a pager:
 * Overview (percentage, counts, [Insights], every class day by day) and [Calculator].
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun CourseDetail(key: String, title: String) {
    val page = rememberPage(
        "attendance_detail",
        { core, flow -> core.cachedAttendanceDetail(flow, key) },
        { core, flow -> core.fetchAttendanceDetail(flow, key) },
        { it.updatedAt },
        item = key,
    ) { it.classes.size }
    val scope = rememberCoroutineScope()
    var tab by rememberSaveable(key) { mutableStateOf(DetailTab.Overview) }
    DisposableEffect(key) { onDispose { StateRegistry.remove("ui.attendance.detail") } }
    // A history fetched in the background while this is open: show it (Rust serves the one
    // just fetched, no second request).
    val arrived = Histories.arrived[key]
    var shownArrival by remember { mutableStateOf(arrived) }
    LaunchedEffect(arrived) {
        if (arrived != shownArrival) {
            shownArrival = arrived
            page.refresh("history", user = false)
        }
    }
    val state = page.state
    val ready = state as? PageState.Ready
    val busy = state is PageState.Loading || ready?.refreshing == true
    // The title, and beside it the refresh (a pull would drag the sheet).
    Row(Modifier.fillMaxWidth().padding(start = Space.xl, end = Space.sm, bottom = Space.xs), verticalAlignment = Alignment.CenterVertically) {
        Text(
            title,
            Modifier.weight(1f),
            style = MaterialTheme.typography.titleMediumEmphasized,
            maxLines = 2,
            overflow = TextOverflow.Ellipsis,
        )
        IconButton(
            { scope.launch { page.refresh("button") } },
            modifier = Modifier.testTag("attendance.detail.refresh"),
            enabled = !busy,
            shapes = IconButtonDefaults.shapes(),
        ) {
            if (busy) LoadingIndicator(Modifier.size(24.dp)) else Icon(Lucide.RefreshCw, "Refresh", Modifier.size(20.dp))
        }
    }
    PageScaffold(state, { page.refresh("retry") }, isEmpty = { false }, empty = {}, pullToRefresh = false) { detail ->
        val course = detail.course
        // A swipe that lands on the other tab selects it.
        val pager = rememberSyncedPager(DetailTab.entries, tab) { landed ->
            tab = landed
            flow("attendance.detail.tab") { id -> Log.info("ui.attendance", "tab ${landed.name.lowercase()} (swipe)", id, mapOf("tab" to landed.name.lowercase(), "by" to "swipe", "course" to course.key)) }
        }
        LaunchedEffect(detail, tab) {
            StateRegistry.report(
                "ui.attendance.detail",
                mapOf(
                    "course" to course.key,
                    "tab" to tab.name.lowercase(),
                    "standing" to course.standing.name,
                    "classes" to detail.classes.size,
                    "notPosted" to detail.classes.count { it.status == null },
                    "plans" to detail.plans.size,
                ),
            )
        }
        Column(Modifier.fillMaxSize().testTag("attendance.detail")) {
            SecondaryTabRow(pager.targetPage, divider = {}) {
                DetailTab.entries.forEach { t ->
                    Tab(
                        selected = pager.targetPage == t.ordinal,
                        onClick = {
                            tab = t
                            scope.launch { pager.animateScrollToPage(t.ordinal) }
                            scope.launch {
                                flow("attendance.detail.tab") { id -> Log.info("ui.attendance", "tab ${t.name.lowercase()}", id, mapOf("tab" to t.name.lowercase(), "by" to "tap", "course" to course.key)) }
                            }
                        },
                        modifier = Modifier.testTag("attendance.detail.tab.${t.name.lowercase()}"),
                        text = { Text(t.label) },
                    )
                }
            }
            HorizontalDivider(color = MaterialTheme.colorScheme.outlineVariant)
            HorizontalPager(pager, Modifier.weight(1f), key = { DetailTab.entries[it].name }) { i ->
                when (DetailTab.entries[i]) {
                    DetailTab.Overview -> Overview(detail)
                    DetailTab.Calculator -> Calculator(course, detail.plans)
                }
            }
        }
    }
}

/** The Overview tab: hero, counts, every class by month, the data's age. */
@Composable
private fun Overview(detail: AttendanceDetail) {
    val course = detail.course
    LazyColumn(
        Modifier.fillMaxSize().testTag("attendance.detail.overview"),
        contentPadding = PaddingValues(
            start = Space.lg,
            top = Space.lg,
            end = Space.lg,
            bottom = LocalContentPadding.current.calculateBottomPadding() + Space.xl,
        ),
        verticalArrangement = Arrangement.spacedBy(Space.lg),
    ) {
        item("hero") { Hero(course) }
        item("stats") { Stats(course) }
        if (detail.classes.any { it.status != null }) item("insights") { Insights(detail.classes) }
        if (detail.classes.isNotEmpty()) item("calendar") { ClassCalendar(detail.classes) }
        classHistory(detail.classes)
        item("updated") { UpdatedFooter() }
    }
}

/** The percentage large, the line in the standing's tone, the advice and any debarment. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Hero(course: AttendanceCourse) {
    val scheme = MaterialTheme.colorScheme
    val standing = course.standing
    val tone = standing.tone()
    SectionCard(Modifier.fillMaxWidth().testTag("attendance.detail.hero")) {
        Text(
            listOf(course.code, course.faculty).filter { it.isNotBlank() }.joinToString(" · "),
            style = MaterialTheme.typography.labelLarge,
            color = scheme.onSurfaceVariant,
            maxLines = 1,
        )
        Row(Modifier.padding(top = Space.xs), verticalAlignment = Alignment.CenterVertically) {
            Text(
                course.percentLabel,
                Modifier.weight(1f),
                style = MaterialTheme.typography.displayMediumEmphasized.nums(),
                color = if (course.debarred) tone.base else scheme.onSurface,
            )
            Column(horizontalAlignment = Alignment.End, verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                KindChip(course.isLab)
            }
        }
        AttendanceLine(course.percent, tone.base, Modifier.padding(vertical = Space.md), thickness = 10.dp)
        val line = course.debarLine() ?: course.advice(showSkip = true)
        line?.let {
            Text(
                it,
                style = MaterialTheme.typography.titleSmallEmphasized.nums(),
                color = if (course.debarred || !course.safe) tone.onSubtle else scheme.onSurfaceVariant,
            )
        }
    }
}

/** Attended, missed and held; labs in sessions. */
@Composable
private fun Stats(course: AttendanceCourse) {
    val (attended, held) = course.sessions
    Row(Modifier.fillMaxWidth().testTag("attendance.detail.stats"), horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
        StatTile("$attended", "Attended", Modifier.weight(1f))
        StatTile("${held - attended}", "Missed", Modifier.weight(1f))
        StatTile("$held", if (course.isLab) "Sessions" else "Classes", Modifier.weight(1f))
    }
}

