package vitapmate.features.grades

import vitapmate.ui.rememberHaptics
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.FileX
import com.composables.icons.lucide.Lucide
import vitapmate.core.GradeHistory
import vitapmate.data.PageHandle
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PageScaffold
import vitapmate.ui.SectionHeader
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.nums

/**
 * The grade history (`page.grade_history`): the CGPA card, then every course by exam session with
 * the session's GPA (of all its courses, whatever the filter); chips show one grade only.
 */
@Composable
internal fun HistoryView(
    page: PageHandle<GradeHistory>,
    filter: String?,
    onFilter: (String?) -> Unit,
    expanded: List<String>,
    onToggle: (String) -> Unit,
    list: LazyListState,
) {
    PageScaffold(
        page.state,
        { page.refresh("pull") },
        isEmpty = { false },
        empty = {},
    ) { history ->
        val sessions = history.sessions.mapNotNull { session ->
            val courses = if (filter == null) session.courses else session.courses.filter { it.grade == filter }
            if (courses.isEmpty()) null else session to courses
        }
        LazyColumn(
            Modifier.fillMaxSize().testTag("grades.list.history"),
            list,
            contentPadding = PaddingValues(
                start = Space.sm,
                top = Space.xs,
                end = Space.sm,
                bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg,
            ),
            // A session's course cards sit 3 dp apart as one segmented run.
            verticalArrangement = Arrangement.spacedBy(3.dp),
        ) {
            item("cgpa") { CgpaCard(history) }
            if (history.courseCount > 0) {
                item("courses") {
                    SectionHeader("Courses", trailing = { Text("${sessions.sumOf { it.second.size }} of ${history.courseCount}") })
                }
                item("filters") {
                    val haptics = rememberHaptics()
                    LazyRow(Modifier.testTag("grades.filter"), horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                        item("all") {
                            FilterChip(filter == null, { haptics.tick(); onFilter(null) }, { Text("All") }, Modifier.testTag("grades.filter.all"))
                        }
                        items(history.filters, key = { it.grade }) { count ->
                            FilterChip(
                                filter == count.grade,
                                { haptics.tick(); onFilter(if (filter == count.grade) null else count.grade) },
                                { Text("${count.grade}  ${count.count}") },
                                Modifier.testTag("grades.filter.${count.grade}"),
                                leadingIcon = { Box(Modifier.size(8.dp).background(gradeTone(count.grade).base, RoundedCornerShape(2.dp))) },
                            )
                        }
                    }
                }
            }
            if (sessions.isEmpty()) {
                item("none") { EmptyView(Lucide.FileX, "No courses yet", Modifier.testTag("grades.history.empty")) }
            }
            sessions.forEach { (session, courses) ->
                item("session.${session.month}") {
                    // The session's GPA in a small pill.
                    SectionHeader(session.label, Modifier.animateItem(), trailing = session.gpa?.let { gpa ->
                        {
                            Text(
                                "GPA ${gpa.fixed2()}",
                                Modifier.background(MaterialTheme.colorScheme.surfaceContainerHigh, RoundedCornerShape(11.dp)).padding(horizontal = 10.dp, vertical = 3.dp),
                                style = MaterialTheme.typography.labelMedium.nums(),
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.onSurface,
                            )
                        }
                    })
                }
                itemsIndexed(courses, key = { _, it -> it.key }) { i, course ->
                    HistoryCourseCard(course, course.key in expanded, { onToggle(course.key) }, Modifier.animateItem(), segment(i, courses.size))
                }
            }
            item("updated") { UpdatedFooter() }
        }
    }
}
