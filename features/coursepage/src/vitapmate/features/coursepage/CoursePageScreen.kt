package vitapmate.features.coursepage

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.BookX
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.LocalOpenDetail
import vitapmate.ui.PageScaffold
import vitapmate.ui.ScrollToTopOnReselect
import vitapmate.ui.SkeletonList
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter

/**
 * The course list (`page.coursepage`, log src `ui.coursepage`): one card per course code in VTOP's
 * order, a button per part. A part opens [CourseScreen] (flow `coursepage.course`).
 */
@Composable
fun CoursePageScreen() {
    val page = rememberPage(
        "coursepage",
        { core, flow -> core.cachedCoursePage(flow) },
        { core, flow -> core.fetchCoursePage(flow) },
        { it.updatedAt },
    ) { data -> data.groups.size }
    val opener = LocalOpenDetail.current
    val scope = rememberCoroutineScope()
    val list = rememberLazyListState()
    ScrollToTopOnReselect(list)
    PageScaffold(
        page.state,
        { page.refresh("pull") },
        isEmpty = { it.groups.isEmpty() },
        empty = { EmptyView(Lucide.BookX, "No courses this semester", Modifier.testTag("coursepage.empty")) },
        loading = { SkeletonList(Modifier.padding(Space.sm), count = 6, height = 116.dp) },
    ) { data ->
        LaunchedEffect(data) {
            StateRegistry.report(
                "ui.coursepage",
                mapOf("courses" to data.groups.size, "parts" to data.groups.sumOf { it.parts.size }, "semester" to data.semesterId),
            )
        }
        LazyColumn(
            Modifier.fillMaxSize().testTag("coursepage.list"),
            list,
            contentPadding = PaddingValues(
                start = Space.sm,
                top = Space.xs,
                end = Space.sm,
                bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg,
            ),
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            items(data.groups, key = { it.code }) { group ->
                CourseCard(
                    group,
                    onOpen = { part ->
                        scope.launch {
                            flow("coursepage.course") { id ->
                                Log.info("ui.coursepage", "open ${part.code} ${part.label}", id, mapOf("course" to part.code, "part" to part.label))
                            }
                        }
                        opener.open(part.id, part.title.ifBlank { part.code })
                    },
                    Modifier.animateItem().testTag("coursepage.course.${group.code}"),
                )
            }
            item("updated") { UpdatedFooter() }
        }
    }
}
