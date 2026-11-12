package vitapmate.features.coursepage

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.LoadingIndicator
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
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
import vitapmate.ui.AppTheme
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PageScaffold
import vitapmate.ui.SectionHeader
import vitapmate.ui.SkeletonList
import vitapmate.ui.Space
import vitapmate.ui.UpdatedFooter
import vitapmate.ui.rememberNow
import vitapmate.ui.tappable

/** Planned lectures shown before "Show all". */
private const val UPCOMING_SHOWN = 3

/**
 * One course part (`page.coursepage_course`, item `<course id>/<picked class id>`): the student's
 * section (by the timetable; else the only one; else a list to pick from) with its header, VTOP
 * files as tiles, and lectures: coming up (the next one alone on the accent fill, the rest joined;
 * three then "Show all"), then covered, newest first, joined. "All lectures" / "With material" filters. Files download into Docs and open
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

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
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
            ClassHeader(view, today, onSwitch = onSwitch.takeIf { view.sections.size > 1 })
        }
        val files = buildList {
            if (plan.syllabusPath.isNotEmpty()) add(FileAction("syllabus", Lucide.FileText, "Syllabus", true) { c, f -> c.courseFileLink(f, plan.syllabusPath) })
            if (plan.hasCoursePlan) add(FileAction("plan", Lucide.Sheet, "Course plan", false) { c, f -> c.coursePlanLink(f, plan.classId) })
            if (plan.generalMaterialsPath.isNotEmpty()) add(FileAction("general", Lucide.FolderArchive, "General material", false) { c, f -> c.courseFileLink(f, plan.generalMaterialsPath) })
            if (plan.allMaterialsPath.isNotEmpty()) add(FileAction("all", Lucide.Download, "All material (zip)", false) { c, f -> c.courseFileLink(f, plan.allMaterialsPath) })
        }
        if (files.isNotEmpty()) item("files") {
            val motion = MaterialTheme.motionScheme
            val accent = AppTheme.tones.accentTone
            // Two tiles to a row, joined: small inner corners, the block's large outer ones.
            Column(
                Modifier.fillMaxWidth().padding(top = Space.xs).clip(MaterialTheme.shapes.extraLarge).testTag("coursepage.files"),
                verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap),
            ) {
                files.chunked(2).forEach { row ->
                    Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                        row.forEach { file ->
                            Row(
                                Modifier.weight(1f).fillMaxHeight().heightIn(min = 60.dp)
                                    .background(AppTheme.colors.card, ListItemDefaults.segmentedShapes(1, 3).shape)
                                    .tappable({ download(file.key, file.key, file.keep, file.fetch) }, file.label, Role.Button, haptic = true)
                                    .testTag("coursepage.file.${file.key}")
                                    .padding(horizontal = 14.dp, vertical = Space.md),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                Box(Modifier.size(36.dp).background(accent.subtle, MaterialTheme.shapes.medium), Alignment.Center) {
                                    AnimatedContent(file.key in busy, transitionSpec = { (fadeIn(motion.fastEffectsSpec()) + scaleIn(motion.fastSpatialSpec())) togetherWith fadeOut(motion.fastEffectsSpec()) }, label = "file-tile") { loading ->
                                        if (loading) LoadingIndicator(Modifier.size(20.dp))
                                        else Icon(file.icon, null, Modifier.size(18.dp), tint = accent.base)
                                    }
                                }
                                Text(
                                    file.label,
                                    Modifier.padding(start = 10.dp),
                                    style = MaterialTheme.typography.labelLarge,
                                    maxLines = 2,
                                    overflow = TextOverflow.Ellipsis,
                                )
                            }
                        }
                    }
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
            // The next lecture stands alone; the rest join into one block.
            next?.let { item("upcoming.next") { LectureItem(it, next, today, planned = true, busy, onMaterial, MaterialTheme.shapes.extraLarge, Modifier.animateItem()) } }
            if (upcoming.size > 1) item("upcoming.rest") {
                JoinedBlock(Modifier.animateItem()) {
                    upcoming.take(UPCOMING_SHOWN).drop(1).forEach { LectureItem(it, next, today, planned = true, busy, onMaterial) }
                    if (upcoming.size > UPCOMING_SHOWN) {
                        AnimatedVisibility(showAll, enter = expandVertically(MaterialTheme.motionScheme.defaultSpatialSpec()) + fadeIn(), exit = shrinkVertically(MaterialTheme.motionScheme.fastSpatialSpec()) + fadeOut()) {
                            JoinedBlock(clip = false) {
                                upcoming.drop(UPCOMING_SHOWN).forEach { LectureItem(it, next, today, planned = true, busy, onMaterial) }
                            }
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
            item("covered") { JoinedBlock(Modifier.animateItem()) { covered.forEach { LectureItem(it, next, today, planned = false, busy, onMaterial) } } }
        }
        item("updated") { UpdatedFooter() }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun LectureItem(
    lecture: Lecture,
    next: Lecture?,
    today: Int,
    planned: Boolean,
    busy: Set<String>,
    onMaterial: (String) -> Unit,
    shape: Shape = ListItemDefaults.segmentedShapes(1, 3).shape,
    modifier: Modifier = Modifier,
) = LectureCard(lecture, today, planned, lecture === next, busy, onMaterial, shape, modifier.testTag("coursepage.lecture.${lecture.serial}"))

/** Lectures as one block: small gaps, the block's large outer corners ([clip]: off for a block inside one). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun JoinedBlock(modifier: Modifier = Modifier, clip: Boolean = true, content: @Composable () -> Unit) = Column(
    modifier.fillMaxWidth().then(if (clip) Modifier.clip(MaterialTheme.shapes.extraLarge) else Modifier),
    verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap),
) { content() }

/** A VTOP file of the section; [keep]: a copy in Docs opens again without fetching. */
private class FileAction(
    val key: String,
    val icon: ImageVector,
    val label: String,
    val keep: Boolean,
    val fetch: suspend (Core, String) -> FileLink,
)
