package vitapmate.features.coursepage

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.Download
import com.composables.icons.lucide.FileText
import com.composables.icons.lucide.FolderArchive
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sheet
import kotlinx.coroutines.launch
import vitapmate.core.Core
import vitapmate.core.CoursePlan
import vitapmate.core.CourseView
import vitapmate.core.Lecture
import vitapmate.core.FileLink
import vitapmate.data.epochDayOf
import vitapmate.data.rememberPage
import vitapmate.observability.StateRegistry
import vitapmate.observability.logAction
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PageScaffold
import vitapmate.ui.SectionHeader
import vitapmate.ui.SkeletonList
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.rememberNow

/** Planned lectures shown before "Show all". */
private const val UPCOMING_SHOWN = 3

/**
 * One course part (`page.coursepage_course`, item `<course id>/<picked class id>`): the student's
 * section (by the timetable; else the only one; else a list to pick from) with its header, VTOP
 * files, and lectures: coming up (the next one marked, three then "Show all"), then covered,
 * newest first. "All lectures" / "With material" filters. Files download into Docs and open
 * ([openCourseFile], flow `coursepage.file`).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun CourseScreen(courseId: String) {
    var picked by rememberSaveable(courseId) { mutableStateOf<String?>(null) }
    val page = rememberPage(
        "coursepage_course",
        { core, flow -> core.cachedCourseView(flow, courseId, picked) },
        { core, flow -> core.fetchCourseView(flow, courseId, picked) },
        { it.updatedAt },
        item = "$courseId/${picked.orEmpty()}",
    ) { view -> view.plan?.lectures?.size ?: view.sections.size }
    var sheet by remember { mutableStateOf(false) }
    val scope = rememberCoroutineScope()
    val pick: (String, String) -> Unit = { classId, by ->
        picked = classId
        sheet = false
        scope.logAction("coursepage.section", "ui.coursepage", "section picked ($by)", mapOf("by" to by))
    }
    PageScaffold(
        page.state,
        { page.refresh("pull") },
        isEmpty = { it.sections.isEmpty() },
        empty = { EmptyView(Lucide.BookOpen, "No sections listed", Modifier.testTag("coursepage.course.empty")) },
        loading = { SkeletonList(Modifier.padding(Space.md), count = 6, height = 80.dp) },
    ) { view ->
        val plan = view.plan
        if (view.selected == null || plan == null) SectionPicker(view) { pick(it, "list") }
        else Plan(view, plan, onSwitch = { sheet = true })
        if (sheet) SectionSheet(view.sections, view.selected, onPick = { pick(it, "sheet") }, onDismiss = { sheet = false })
    }
}

/** The sections to pick from, under the course's code and title. */
@Composable
private fun SectionPicker(view: CourseView, onPick: (String) -> Unit) {
    LaunchedEffect(view) { StateRegistry.report("ui.coursepage.course", mapOf("code" to view.course.code, "sections" to view.sections.size, "selected" to null)) }
    LazyColumn(
        Modifier.fillMaxSize().testTag("coursepage.sections"),
        contentPadding = PaddingValues(start = Space.sm, top = Space.sm, end = Space.sm, bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg),
        verticalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        item("title") { CourseTitle(view.course, large = true) }
        item("header") { SectionHeader("Sections", trailing = { Text("${view.sections.size}") }) }
        items(view.sections, key = { it.classId }) { section ->
            SectionRow(section, selected = false, onPress = { onPick(section.classId) }, Modifier.animateItem())
        }
        item("updated") { UpdatedFooter() }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun Plan(view: CourseView, plan: CoursePlan, onSwitch: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val now by rememberNow()
    val today = epochDayOf(now)
    var withMaterialOnly by rememberSaveable(plan.classId) { mutableStateOf(false) }
    var showAll by rememberSaveable(plan.classId) { mutableStateOf(false) }
    // Paths (or file kinds) being fetched, for their spinners.
    var busy by remember { mutableStateOf(setOf<String>()) }
    val download: (key: String, kind: String, keep: Boolean, fetch: suspend (Core, String) -> FileLink) -> Unit = { key, kind, keep, fetch ->
        if (key !in busy) {
            busy = busy + key
            scope.launch {
                try {
                    openCourseFile(context, "${plan.classId}:$key", kind, keep, fetch)
                } finally {
                    busy = busy - key
                }
            }
        }
    }
    val lectures = plan.lectures.filter { !withMaterialOnly || it.materials.isNotEmpty() }
    val upcoming = lectures.filter { (it.day ?: Int.MIN_VALUE) >= today }
    val covered = lectures.filter { (it.day ?: Int.MIN_VALUE) < today }.reversed()
    val next = upcoming.firstOrNull()
    val withMaterial = plan.lectures.count { it.materials.isNotEmpty() }
    LaunchedEffect(view, withMaterialOnly, showAll, today) {
        StateRegistry.report(
            "ui.coursepage.course",
            mapOf(
                "code" to view.course.code,
                "part" to view.course.label,
                "sections" to view.sections.size,
                "mine" to (view.sections.firstOrNull { it.classId == view.selected }?.mine == true),
                "lectures" to plan.lectures.size,
                "withMaterial" to withMaterial,
                "upcoming" to upcoming.size,
                "covered" to covered.size,
                "filter" to if (withMaterialOnly) "material" else "all",
                "showAll" to showAll,
                "files" to listOf("syllabus" to plan.syllabusPath.isNotEmpty(), "plan" to plan.hasCoursePlan, "general" to plan.generalMaterialsPath.isNotEmpty(), "all" to plan.allMaterialsPath.isNotEmpty())
                    .filter { it.second }.map { it.first },
            ),
        )
    }
    val list = rememberLazyListState()
    LazyColumn(
        Modifier.fillMaxSize().testTag("coursepage.plan"),
        list,
        contentPadding = PaddingValues(start = Space.sm, top = Space.sm, end = Space.sm, bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg),
        verticalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        item("header") {
            ClassHeader(view, onSwitch = onSwitch.takeIf { view.sections.size > 1 })
        }
        val files = buildList {
            if (plan.syllabusPath.isNotEmpty()) add(FileAction("syllabus", Lucide.FileText, "Syllabus", true) { c, f -> c.courseFileLink(f, plan.syllabusPath) })
            if (plan.hasCoursePlan) add(FileAction("plan", Lucide.Sheet, "Course plan", false) { c, f -> c.coursePlanLink(f, plan.classId) })
            if (plan.generalMaterialsPath.isNotEmpty()) add(FileAction("general", Lucide.FolderArchive, "General material", false) { c, f -> c.courseFileLink(f, plan.generalMaterialsPath) })
            if (plan.allMaterialsPath.isNotEmpty()) add(FileAction("all", Lucide.Download, "All material (zip)", false) { c, f -> c.courseFileLink(f, plan.allMaterialsPath) })
        }
        if (files.isNotEmpty()) item("files") {
            FlowRow(Modifier.fillMaxWidth().padding(top = Space.xs).testTag("coursepage.files"), horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                files.forEach { file ->
                    FileChip(file.icon, file.label, file.key in busy, { download(file.key, file.key, file.keep, file.fetch) }, Modifier.testTag("coursepage.file.${file.key}"))
                }
            }
        }
        item("filter") {
            ConnectedChoice(
                listOf(false, true),
                withMaterialOnly,
                onSelect = { only ->
                    withMaterialOnly = only
                    val filter = if (only) "material" else "all"
                    scope.logAction("coursepage.filter", "ui.coursepage", "filter $filter", mapOf("filter" to filter))
                },
                tag = "coursepage.filter",
                key = { if (it) "material" else "all" },
                label = { if (it) "With material ($withMaterial)" else "All lectures (${plan.lectures.size})" },
                modifier = Modifier.padding(top = Space.sm),
            )
        }
        if (lectures.isEmpty()) item("empty") { EmptyView(Lucide.BookOpen, "Nothing here yet", Modifier.fillMaxWidth().testTag("coursepage.lectures.empty")) }
        val onMaterial: (String) -> Unit = { path -> download(path, "material", true) { c, f -> c.courseFileLink(f, path) } }
        if (upcoming.isNotEmpty()) {
            item("upcoming.header") { SectionHeader("Coming up", Modifier.animateItem(), trailing = { Text("${upcoming.size} planned") }) }
            lectureItems("upcoming", upcoming.take(UPCOMING_SHOWN), next, today, planned = true, busy, onMaterial)
            if (upcoming.size > UPCOMING_SHOWN) item("upcoming.more") {
                AnimatedVisibility(showAll, enter = expandVertically(MaterialTheme.motionScheme.defaultSpatialSpec()) + fadeIn(), exit = shrinkVertically(MaterialTheme.motionScheme.fastSpatialSpec()) + fadeOut()) {
                    Column(verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                        upcoming.drop(UPCOMING_SHOWN).forEach { lecture ->
                            LectureCard(lecture, today, planned = true, next = false, busy, onMaterial)
                        }
                    }
                }
            }
            if (upcoming.size > UPCOMING_SHOWN) item("upcoming.toggle") {
                TextButton(
                    {
                        showAll = !showAll
                        scope.logAction("coursepage.planned", "ui.coursepage", if (showAll) "show all planned" else "show fewer planned")
                    },
                    Modifier.fillMaxWidth().animateItem().testTag("coursepage.upcoming.toggle"),
                ) { Text(if (showAll) "Show fewer" else "Show all ${upcoming.size} planned") }
            }
        }
        if (covered.isNotEmpty()) {
            item("covered.header") { SectionHeader("Covered", Modifier.animateItem(), trailing = { Text("${covered.size}") }) }
            lectureItems("covered", covered, next, today, planned = false, busy, onMaterial)
        }
        item("updated") { UpdatedFooter() }
    }
}

private fun LazyListScope.lectureItems(
    group: String,
    lectures: List<Lecture>,
    next: Lecture?,
    today: Int,
    planned: Boolean,
    busy: Set<String>,
    onMaterial: (String) -> Unit,
) = items(lectures, key = { "$group.${it.serial}.${it.day}" }) { lecture ->
    LectureCard(lecture, today, planned, lecture === next, busy, onMaterial, Modifier.animateItem().testTag("coursepage.lecture.${lecture.serial}"))
}

/** A VTOP file of the section; [keep]: a copy in Docs opens again without fetching. */
private class FileAction(
    val key: String,
    val icon: ImageVector,
    val label: String,
    val keep: Boolean,
    val fetch: suspend (Core, String) -> FileLink,
)
