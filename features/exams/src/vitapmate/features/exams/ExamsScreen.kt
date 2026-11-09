package vitapmate.features.exams

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyListState
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
import com.composables.icons.lucide.CalendarX
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.data.epochDayOf
import vitapmate.data.minuteOf
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.EmptyView
import vitapmate.ui.PageScaffold
import vitapmate.ui.ScrollToTopOnReselect
import vitapmate.ui.Space
import vitapmate.ui.rememberNow
import vitapmate.ui.rememberSyncedPager

/**
 * Exams: the saved schedule first, then VTOP (`page.exams`, log src `ui.exams`). With several
 * exams, CAT-I / CAT-II / FAT over a pager (swipe or tap), opening on the next one; each page is
 * a [ExamTimeline]. A finished paper folds to one row and taps open again. Pull to fetch again;
 * tapping the tab again scrolls the visible list to the top.
 */
@Composable
fun ExamsScreen() {
    val page = rememberPage(
        "exams",
        { core, flow -> core.cachedExams(flow) },
        { core, flow -> core.fetchExams(flow) },
        { it.updatedAt },
    ) { exams -> exams.groups.sumOf { it.papers.size } }
    val scope = rememberCoroutineScope()
    // The exam picked by name, so a refresh that adds one keeps the pick; null = Rust's `opens`.
    var picked by rememberSaveable { mutableStateOf<String?>(null) }
    // Finished papers opened again, by `<exam>/<key>`.
    var expanded by rememberSaveable { mutableStateOf(listOf<String>()) }
    val now by rememberNow()
    PageScaffold(
        page.state,
        { page.refresh("pull") },
        isEmpty = { it.groups.isEmpty() },
        empty = { EmptyView(Lucide.CalendarX, "No exam schedule yet", Modifier.testTag("exams.empty")) },
    ) { data ->
        val groups = data.groups
        val today = epochDayOf(now)
        val minute = minuteOf(now)
        val start = groups.indexOfFirst { it.name == picked }.takeIf { it >= 0 } ?: data.opens.coerceIn(0, groups.lastIndex)
        // A swipe that lands on another exam picks it.
        val pager = rememberSyncedPager(groups.map { it.name }, groups[start].name) { name ->
            picked = name
            flow("exams.filter") { id -> Log.info("ui.exams", "exam $name (swipe)", id, mapOf("exam" to name, "by" to "swipe")) }
        }
        val lists = remember(groups.size) { List(groups.size) { LazyListState() } }
        ScrollToTopOnReselect(lists[pager.currentPage.coerceIn(0, groups.lastIndex)])
        val group = groups[pager.settledPage.coerceIn(0, groups.lastIndex)]
        LaunchedEffect(data, group, expanded, today) {
            StateRegistry.report(
                "ui.exams",
                mapOf(
                    "exam" to group.name,
                    "exams" to groups.map { it.name },
                    "papers" to groups.sumOf { it.papers.size },
                    "shown" to group.papers.size,
                    "ahead" to group.papers.count { !it.done(today, minute) },
                    "undated" to group.papers.count { it.day == null },
                    "next" to group.papers.firstOrNull { !it.done(today, minute) }?.key,
                    "expanded" to expanded,
                ),
            )
        }
        Column(Modifier.fillMaxSize()) {
            if (groups.size > 1) {
                ConnectedChoice(
                    groups.map { it.name },
                    groups[pager.targetPage.coerceIn(0, groups.lastIndex)].name,
                    onSelect = { name ->
                        picked = name
                        scope.launch { pager.animateScrollToPage(groups.indexOfFirst { it.name == name }) }
                        scope.logAction("exams.filter", "ui.exams", "exam $name", mapOf("exam" to name, "by" to "tap"))
                    },
                    tag = "exams.filter",
                    key = { it.tagKey() },
                    label = { it },
                    modifier = Modifier.padding(start = Space.sm, top = Space.xs, end = Space.sm, bottom = Space.sm),
                )
            }
            HorizontalPager(pager, Modifier.weight(1f).testTag("exams.pager"), key = { groups[it].name }) { i ->
                val g = groups[i]
                ExamTimeline(
                    g,
                    lists[i],
                    today,
                    minute,
                    expanded,
                    onToggle = { paper ->
                        val id = "${g.name}/${paper.key}"
                        val open = id !in expanded
                        expanded = if (open) expanded + id else expanded - id
                        scope.launch {
                            val what = if (open) "expand" else "collapse"
                            flow("exams.$what") { f -> Log.info("ui.exams", "$what ${paper.key}", f, mapOf("exam" to g.name, "paper" to paper.key)) }
                        }
                    },
                    Modifier.testTag("exams.list.${g.name.tagKey()}"),
                )
            }
        }
    }
}

/** "CAT-II" → "cat-ii", for test tags. */
internal fun String.tagKey() = lowercase().replace(' ', '-')
