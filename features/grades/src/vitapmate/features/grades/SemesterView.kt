package vitapmate.features.grades

import vitapmate.ui.rememberHaptics
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.School
import kotlinx.coroutines.launch
import vitapmate.core.GradeHistory
import vitapmate.core.Grades
import vitapmate.core.semesterSummary
import vitapmate.data.PageState
import vitapmate.data.SemesterChoice
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.logAction
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PageScaffold
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.shortSemesterName

/**
 * One semester's grades (`page.grades`, cached per semester): [viewed], or with none the picked
 * semester's, falling back to the one before while the picked one has none yet (as Flutter opens
 * the page). Chips pick any semester VTOP lists; the summary's GPA takes credits from [history].
 * A card expands in place to its breakdown ([Details]). [shown] reports the semester on screen.
 */
@Composable
internal fun SemesterView(
    viewed: String,
    onView: (String) -> Unit,
    history: GradeHistory?,
    details: Details,
    expanded: List<String>,
    onToggle: (String) -> Unit,
    list: LazyListState,
    shown: (Grades?) -> Unit,
) {
    val semester = viewed
    val page = rememberPage(
        "grades",
        { core, flow -> core.cachedGrades(flow, semester) },
        { core, flow -> core.fetchGrades(flow, semester) },
        { it.updatedAt },
        item = semester,
    ) { it.courses.size }
    val scope = rememberCoroutineScope()
    val data = (page.state as? PageState.Ready)?.data
    LaunchedEffect(data) { shown(data) }
    Column(Modifier.fillMaxSize()) {
        SemesterChips(data?.semesterId ?: semester.ifEmpty { SemesterChoice.selected.orEmpty() }) { id ->
            onView(id)
            scope.logAction("grades.semester", "ui.grades", "semester $id", mapOf("semester" to id))
        }
        PageScaffold(
            page.state,
            { page.refresh("pull") },
            isEmpty = { it.courses.isEmpty() },
            empty = { EmptyView(Lucide.School, "No grades for this semester", Modifier.testTag("grades.empty")) },
            modifier = Modifier.weight(1f),
        ) { grades ->
            val summary = remember(grades, history) { semesterSummary(grades.courses, history) }
            LazyColumn(
                Modifier.fillMaxSize().testTag("grades.list.semester"),
                list,
                contentPadding = PaddingValues(
                    start = Space.sm,
                    top = Space.xs,
                    end = Space.sm,
                    bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg,
                ),
                verticalArrangement = Arrangement.spacedBy(Space.sm),
            ) {
                item("summary") {
                    SemesterSummaryCard(grades.courses, summary, onCourse = { i -> scope.launch { list.animateScrollToItem(i + 1) } }, Modifier.animateItem())
                }
                items(grades.courses, key = { it.courseId.ifEmpty { it.code } }) { course ->
                    val key = "${grades.semesterId}/${course.courseId}"
                    val open = key in expanded
                    LaunchedEffect(open) { if (open) details.open(grades.semesterId, course.courseId) }
                    GradeCard(
                        course,
                        open,
                        details.loaded[key],
                        key in details.loading,
                        key in details.failed,
                        onToggle = { onToggle(key) },
                        onRefresh = { scope.launch { details.refresh(grades.semesterId, course.courseId) } },
                        Modifier.animateItem(),
                    )
                }
                item("updated") { UpdatedFooter() }
            }
        }
    }
}

/** Every semester VTOP lists, newest first, as a row of chips; [selected] filled. */
@Composable
private fun SemesterChips(selected: String, onSelect: (String) -> Unit) {
    val semesters = SemesterChoice.semesters?.items.orEmpty()
    if (semesters.isEmpty()) return
    val row = rememberLazyListState()
    LaunchedEffect(selected, semesters) {
        val at = semesters.indexOfFirst { it.id == selected }
        if (at > 0) row.animateScrollToItem(at)
    }
    LazyRow(
        Modifier.testTag("grades.semesters"),
        row,
        contentPadding = PaddingValues(horizontal = Space.sm, vertical = Space.xs),
        horizontalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        itemsIndexed(semesters, key = { _, it -> it.id }) { _, semester ->
            val haptics = rememberHaptics()
            FilterChip(
                selected = semester.id == selected,
                onClick = { haptics.tick(); if (semester.id != selected) onSelect(semester.id) },
                label = { Text(listOf(shortSemesterName(semester), semester.note).filter { it.isNotBlank() }.joinToString(" "), maxLines = 1) },
                modifier = Modifier.testTag("grades.semester.${semester.id}"),
            )
        }
    }
}
