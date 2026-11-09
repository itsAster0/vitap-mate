package vitapmate.features.grades

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
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
import com.composables.icons.lucide.Calculator
import com.composables.icons.lucide.GraduationCap
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.core.Grades
import vitapmate.data.PageState
import vitapmate.data.SemesterChoice
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.ScrollToTopOnReselect
import vitapmate.ui.Space
import vitapmate.ui.rememberSyncedPager

/** The three views, as the Flutter app's Grades, Grade History and GPA Planner pages. */
private enum class View { Semester, History, Planner }

/**
 * Grades (log src `ui.grades`): Semester / History / Planner over a pager (swipe or tap). The
 * grade history (`page.grade_history`, per account) is read here for all three: the semester's
 * GPA takes its credits, the planner its CGPA. Tapping the tab again scrolls the visible list up.
 */
@Composable
fun GradesScreen() {
    val history = rememberPage(
        "grade_history",
        { core, flow -> core.cachedGradeHistory(flow) },
        { core, flow -> core.fetchGradeHistory(flow) },
        { it.updatedAt },
    ) { it.courseCount }
    val scope = rememberCoroutineScope()
    var view by rememberSaveable { mutableStateOf(View.Semester) }
    // The semester shown; empty: the picked one (or the one before while it has no grades).
    var viewed by rememberSaveable(SemesterChoice.selected) { mutableStateOf("") }
    var expanded by rememberSaveable { mutableStateOf(listOf<String>()) }
    var historyOpen by rememberSaveable { mutableStateOf(listOf<String>()) }
    var filter by rememberSaveable { mutableStateOf<String?>(null) }
    var shown by remember { mutableStateOf<Grades?>(null) }
    val details = remember { Details() }
    val planner = remember { Planner() }
    val pager = rememberSyncedPager(View.entries, view) { to ->
        view = to
        flow("grades.view") { id -> Log.info("ui.grades", "view ${to.name.lowercase()} (swipe)", id, mapOf("view" to to.name.lowercase(), "by" to "swipe")) }
    }
    val lists = View.entries.map { rememberLazyListState() }
    ScrollToTopOnReselect(lists[pager.currentPage])
    val historyData = (history.state as? PageState.Ready)?.data
    LaunchedEffect(view, shown, expanded, filter, historyData, planner.rows.size) {
        StateRegistry.report(
            "ui.grades",
            mapOf(
                "view" to view.name.lowercase(),
                "semester" to shown?.semesterId,
                "viewed" to viewed,
                "courses" to shown?.courses?.size,
                "expanded" to expanded,
                "details" to details.loaded.keys.toList(),
                "historyCourses" to historyData?.courseCount,
                "sessions" to historyData?.sessions?.size,
                "filter" to filter,
                "planRows" to planner.rows.size,
            ),
        )
    }
    Column(Modifier.fillMaxSize()) {
        ConnectedChoice(
            View.entries,
            View.entries[pager.targetPage],
            onSelect = { to ->
                view = to
                scope.launch { pager.animateScrollToPage(to.ordinal) }
                scope.logAction("grades.view", "ui.grades", "view ${to.name.lowercase()}", mapOf("view" to to.name.lowercase(), "by" to "tap"))
            },
            tag = "grades.view",
            key = { it.name.lowercase() },
            label = { it.name },
            icon = {
                when (it) {
                    View.Semester -> Lucide.GraduationCap
                    View.History -> Lucide.History
                    View.Planner -> Lucide.Calculator
                }
            },
            modifier = Modifier.padding(start = Space.sm, top = Space.xs, end = Space.sm, bottom = Space.xs),
            labelOnSelected = true,
        )
        HorizontalPager(pager, Modifier.weight(1f).testTag("grades.pager"), key = { View.entries[it].name }) { i ->
            when (View.entries[i]) {
                View.Semester -> SemesterView(
                    viewed,
                    onView = { viewed = it },
                    history = historyData,
                    details = details,
                    expanded = expanded,
                    onToggle = { key ->
                        val open = key !in expanded
                        expanded = if (open) expanded + key else expanded - key
                        scope.launch {
                            val what = if (open) "expand" else "collapse"
                            flow("grades.$what") { id -> Log.info("ui.grades", "$what $key", id, mapOf("course" to key)) }
                        }
                    },
                    list = lists[i],
                    shown = { shown = it },
                )
                View.History -> HistoryView(
                    history,
                    filter,
                    onFilter = { to ->
                        filter = to
                        scope.logAction("grades.filter", "ui.grades", "filter ${to ?: "all"}", mapOf("filter" to (to ?: "all")))
                    },
                    expanded = historyOpen,
                    onToggle = { key ->
                        val open = key !in historyOpen
                        historyOpen = if (open) historyOpen + key else historyOpen - key
                        scope.launch {
                            val what = if (open) "expand" else "collapse"
                            flow("grades.history.$what") { id -> Log.info("ui.grades", "history $what $key", id, mapOf("course" to key)) }
                        }
                    },
                    list = lists[i],
                )
                View.Planner -> PlannerView(planner, historyData, lists[i])
            }
        }
    }
}
