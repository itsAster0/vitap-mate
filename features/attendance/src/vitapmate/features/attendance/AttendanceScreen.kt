package vitapmate.features.attendance

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import com.composables.icons.lucide.CalendarMinus
import androidx.compose.ui.Alignment
import androidx.compose.material3.Icon
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.ClipboardList
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendancePlan
import vitapmate.data.CalendarRefresh
import vitapmate.data.Histories
import vitapmate.data.PageState
import vitapmate.data.Settings
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PageScaffold
import vitapmate.ui.ScrollToTopOnReselect
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.rememberSyncedPager

/**
 * Attendance: the saved copy first, then VTOP (`page.attendance`, log src `ui.attendance`).
 * With both kinds, the All/Theory/Lab filter over a pager of the three lists (swipe or tap);
 * a card opens the course in a sheet (kept across rotation and process death). Pull to fetch again; tapping the tab again scrolls the
 * visible list to the top.
 */
@Composable
fun AttendanceScreen() {
    val page = rememberPage(
        "attendance",
        { core, flow -> core.cachedAttendance(flow) },
        { core, flow -> core.fetchAttendance(flow) },
        { it.updatedAt },
    ) { it.courses.size }
    val scope = rememberCoroutineScope()
    var filter by rememberSaveable { mutableStateOf(Kind.All) }
    var open by rememberSaveable { mutableStateOf<String?>(null) }
    var skipDay by rememberSaveable { mutableStateOf(false) }
    // After each fetch of the summary, histories that fell behind it are fetched again in the
    // background, and the calendar when a day old; the refresh spinner doesn't wait. A newer
    // refresh cancels the last run.
    val fresh = (page.state as? PageState.Ready)?.takeIf { !it.refreshing }?.updatedAt
    var seen by remember { mutableStateOf<Long?>(null) }
    LaunchedEffect(fresh) {
        if (fresh == null) return@LaunchedEffect
        val before = seen
        seen = fresh
        if (before == null || before == fresh) return@LaunchedEffect
        (page.state as? PageState.Ready)?.data?.semesterId?.let { launch { CalendarRefresh.ifOld(it, "attendance", anyway = true) } }
        Histories.sync("attendance") { c, id -> c.mismatchedAttendanceDetails(id) }
    }
    PageScaffold(
        page.state,
        { page.refresh("pull") },
        isEmpty = { it.courses.isEmpty() },
        empty = { EmptyView(Lucide.ClipboardList, "No attendance yet", Modifier.testTag("attendance.empty")) },
    ) { data ->
        // Read again when a history lands too: what is held but not posted yet counts from it.
        val outlook = rememberOutlook(data to Histories.arrived.values.sum())
        val labs = data.courses.count { it.isLab }
        // With one kind only there is no filter and no pager.
        val pages = if (labs in 1..<data.courses.size) Kind.entries else listOf(Kind.All)
        // A swipe that lands on another list moves the filter with it.
        val pager = rememberSyncedPager(pages, filter) { kind ->
            filter = kind
            flow("attendance.filter") { id -> Log.info("ui.attendance", "filter ${kind.name.lowercase()} (swipe)", id, mapOf("filter" to kind.name.lowercase(), "by" to "swipe")) }
        }
        // One list state per kind, kept across refreshes and process death.
        val lists = Kind.entries.map { rememberLazyListState() }
        ScrollToTopOnReselect(lists[pages[pager.currentPage.coerceIn(0, pages.lastIndex)].ordinal])
        LaunchedEffect(pager.settledPage) {
            val kind = pages.getOrNull(pager.settledPage) ?: return@LaunchedEffect
            StateRegistry.report("ui.attendance", mapOf("filter" to kind.name.lowercase(), "shown" to data.courses.of(kind).size, "debarred" to data.courses.count { it.debarred }))
        }
        val canSkipDay = outlook.values.any { it.days.isNotEmpty() }
        Column(Modifier.fillMaxSize()) {
            if (pages.size > 1 || canSkipDay) Row(
                Modifier.padding(start = Space.sm, top = Space.xs, end = Space.sm, bottom = Space.sm),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (pages.size > 1) KindFilter(
                    pages[pager.targetPage],
                    mapOf(Kind.All to data.courses.size, Kind.Theory to data.courses.size - labs, Kind.Lab to labs),
                    onSelect = { kind ->
                        filter = kind
                        scope.launch { pager.animateScrollToPage(pages.indexOf(kind)) }
                        scope.launch {
                            flow("attendance.filter") { id ->
                                Log.info("ui.attendance", "filter ${kind.name.lowercase()}", id, mapOf("filter" to kind.name.lowercase(), "by" to "tap"))
                            }
                        }
                    },
                    Modifier.weight(1f),
                ) else Spacer(Modifier.weight(1f))
                if (canSkipDay) {
                    FilledTonalIconButton(
                        {
                            skipDay = true
                            scope.logAction("attendance.skip_day.open", "ui.attendance", "open skip days")
                        },
                        Modifier.padding(start = Space.sm).testTag("attendance.skip_day.open"),
                    ) { Icon(Lucide.CalendarMinus, "Skip days") }
                }
            }
            HorizontalPager(pager, Modifier.weight(1f).testTag("attendance.pager"), key = { pages[it].name }) { i ->
                CourseList(data.courses.of(pages[i]), lists[pages[i].ordinal], outlook, { open = it }, Modifier.testTag("attendance.list.${pages[i].name.lowercase()}"))
            }
        }
        if (skipDay) SkipDaySheet(data.courses, outlook) { skipDay = false }
        // A course gone after a refresh closes its sheet.
        data.courses.firstOrNull { it.key == open }?.let { course ->
            CourseSheet(course.key, course.name.ifBlank { course.code }) {
                open = null
                scope.logAction("attendance.detail.close", "ui.attendance", "close ${course.key}", mapOf("course" to course.key))
            }
        }
    }
}

private fun List<AttendanceCourse>.of(kind: Kind) = when (kind) {
    Kind.All -> this
    Kind.Theory -> filter { !it.isLab }
    Kind.Lab -> filter { it.isLab }
}

/** One page of the pager: a card per course, then the data's age. */
@Composable
private fun CourseList(courses: List<AttendanceCourse>, state: LazyListState, outlook: Map<String, AttendancePlan>, onOpen: (String) -> Unit, modifier: Modifier) {
    val scope = rememberCoroutineScope()
    val showSkip = Settings.showSkipAdvice
    LazyColumn(
        modifier.fillMaxSize(),
        state,
        contentPadding = PaddingValues(
            start = Space.sm,
            top = Space.xs,
            end = Space.sm,
            bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg,
        ),
        verticalArrangement = Arrangement.spacedBy(Space.md),
    ) {
        items(courses, key = { it.key }) { course ->
            CourseCard(
                course,
                outlook[course.key],
                showSkip,
                onClick = {
                    onOpen(course.key)
                    scope.launch {
                        flow("attendance.open_course") { id ->
                            Log.info(
                                "ui.attendance",
                                "open ${course.key}",
                                id,
                                mapOf("course" to course.key, "standing" to course.standing.name, "total" to "${course.total}"),
                            )
                        }
                    }
                },
                Modifier.animateItem(),
            )
        }
        item("updated") { UpdatedFooter() }
    }
}
