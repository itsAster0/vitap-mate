package vitapmate.features.marks

import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.LayoutGrid
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.ClipboardList
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.core.MarksCourse
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PageScaffold
import vitapmate.ui.ScrollToTopOnReselect
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.rememberSyncedPager

/** Which courses the list shows. */
private enum class Kind { All, Theory, Lab }

private fun List<MarksCourse>.of(kind: Kind) = when (kind) {
    Kind.All -> this
    Kind.Theory -> filter { !it.isLab }
    Kind.Lab -> filter { it.isLab }
}

/**
 * Marks: the saved copy first, then VTOP (`page.marks`, log src `ui.marks`). With both kinds,
 * All / Theory / Lab over a pager of the three lists (swipe or tap); a card expands in place to
 * its assessments (several may be open). Pull to fetch again; tapping the tab again scrolls the visible list to the top.
 */
@Composable
fun MarksScreen() {
    val page = rememberPage(
        "marks",
        { core, flow -> core.cachedMarks(flow) },
        { core, flow -> core.fetchMarks(flow) },
        { it.updatedAt },
    ) { it.courses.size }
    val scope = rememberCoroutineScope()
    var filter by rememberSaveable { mutableStateOf(Kind.All) }
    // Open cards by key; several may be open, kept across filters, refreshes and process death.
    var expanded by rememberSaveable { mutableStateOf(listOf<String>()) }
    PageScaffold(
        page.state,
        { page.refresh("pull") },
        isEmpty = { it.courses.isEmpty() },
        empty = { EmptyView(Lucide.ClipboardList, "No marks yet", Modifier.testTag("marks.empty")) },
    ) { data ->
        val labs = data.courses.count { it.isLab }
        // With one kind only there is no filter and no pager.
        val pages = if (labs in 1..<data.courses.size) Kind.entries else listOf(Kind.All)
        // A swipe that lands on another list moves the filter with it.
        val pager = rememberSyncedPager(pages, filter) { kind ->
            filter = kind
            flow("marks.filter") { id -> Log.info("ui.marks", "filter ${kind.name.lowercase()} (swipe)", id, mapOf("filter" to kind.name.lowercase(), "by" to "swipe")) }
        }
        val lists = Kind.entries.map { rememberLazyListState() }
        ScrollToTopOnReselect(lists[pages[pager.currentPage.coerceIn(0, pages.lastIndex)].ordinal])
        LaunchedEffect(data, filter, expanded) {
            StateRegistry.report(
                "ui.marks",
                mapOf(
                    "filter" to filter.name.lowercase(),
                    "shown" to data.courses.of(filter).size,
                    "courses" to data.courses.size,
                    "assessments" to data.courses.sumOf { it.assessments.size },
                    "assessed" to data.courses.count { it.maxWeighted > 0 },
                    "expanded" to expanded,
                ),
            )
        }
        Column(Modifier.fillMaxSize()) {
            if (pages.size > 1) {
                val counts = mapOf(Kind.All to data.courses.size, Kind.Theory to data.courses.size - labs, Kind.Lab to labs)
                ConnectedChoice(
                    Kind.entries,
                    pages[pager.targetPage.coerceIn(0, pages.lastIndex)],
                    onSelect = { kind ->
                        filter = kind
                        scope.launch { pager.animateScrollToPage(pages.indexOf(kind)) }
                        scope.launch {
                            flow("marks.filter") { id -> Log.info("ui.marks", "filter ${kind.name.lowercase()}", id, mapOf("filter" to kind.name.lowercase(), "by" to "tap")) }
                        }
                    },
                    tag = "marks.filter",
                    key = { it.name.lowercase() },
                    label = { "${it.name} ${counts[it] ?: 0}" },
                    icon = { when (it) { Kind.All -> Lucide.LayoutGrid; Kind.Theory -> Lucide.BookOpen; Kind.Lab -> Lucide.FlaskConical } },
                    labelOnSelected = true,
                    modifier = Modifier.padding(start = Space.sm, top = Space.xs, end = Space.sm, bottom = Space.sm),
                )
            }
            HorizontalPager(pager, Modifier.weight(1f).testTag("marks.pager"), key = { pages[it].name }) { i ->
                CourseList(
                    data.courses.of(pages[i]),
                    lists[pages[i].ordinal],
                    expanded,
                    onToggle = { course ->
                        val open = course.key !in expanded
                        expanded = if (open) expanded + course.key else expanded - course.key
                        scope.launch {
                            val what = if (open) "expand" else "collapse"
                            flow("marks.$what") { id ->
                                Log.info("ui.marks", "$what ${course.key}", id, mapOf("course" to course.key, "assessments" to "${course.assessments.size}", "band" to course.band.name))
                            }
                        }
                    },
                    Modifier.testTag("marks.list.${pages[i].name.lowercase()}"),
                )
            }
        }
    }
}

/** One page of the pager: a card per course, then the data's age. */
@Composable
private fun CourseList(courses: List<MarksCourse>, state: LazyListState, expanded: List<String>, onToggle: (MarksCourse) -> Unit, modifier: Modifier) {
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
            MarksCard(course, course.key in expanded, onToggle = { onToggle(course) }, Modifier.animateItem())
        }
        item("updated") { UpdatedFooter() }
    }
}
