package vitapmate.features.attendance

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.LoadingIndicator
import androidx.compose.ui.text.style.TextOverflow
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Calculator
import com.composables.icons.lucide.List
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
import androidx.compose.ui.unit.sp
import androidx.compose.ui.layout.layout
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.withStyle
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import kotlin.math.roundToInt
import kotlinx.coroutines.launch
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendanceDetail
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PageScaffold
import vitapmate.ui.AppTheme
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.KindChip
import vitapmate.ui.Space
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
            ConnectedChoice(
                DetailTab.entries,
                DetailTab.entries[pager.targetPage],
                { t ->
                    tab = t
                    scope.launch { pager.animateScrollToPage(t.ordinal) }
                    scope.launch {
                        flow("attendance.detail.tab") { id -> Log.info("ui.attendance", "tab ${t.name.lowercase()}", id, mapOf("tab" to t.name.lowercase(), "by" to "tap", "course" to course.key)) }
                    }
                },
                tag = "attendance.detail.tab",
                key = { it.name.lowercase() },
                label = { it.label },
                icon = { if (it == DetailTab.Overview) Lucide.List else Lucide.Calculator },
                modifier = Modifier.padding(horizontal = Space.lg, vertical = Space.xs),
            )
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
        if (detail.classes.any { it.status != null }) item("insights") { Insights(detail.classes) }
        if (detail.classes.isNotEmpty()) item("calendar") { ClassCalendar(detail.classes) }
        classHistory(detail.classes)
        item("updated") { UpdatedFooter() }
    }
}

/**
 * Where the course stands, on the standing's quiet fill: code, faculty and kind; the percentage
 * very large beside the advice or debarment and the standing; the line with its 75% mark; then
 * attended, missed and held (labs in sessions) as connected cells.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Hero(course: AttendanceCourse) {
    val standing = course.standing
    val tone = standing.tone()
    val ink = tone.onSubtle
    val card = AppTheme.colors.card
    Column(
        Modifier.fillMaxWidth().background(tone.subtle, RoundedCornerShape(32.dp)).padding(start = 6.dp, end = 6.dp, top = 18.dp, bottom = Space.md)
            .testTag("attendance.detail.hero"),
    ) {
        Column(Modifier.padding(horizontal = Space.md)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    listOf(course.code, course.faculty).filter { it.isNotBlank() }.joinToString(" · "),
                    Modifier.weight(1f),
                    style = MaterialTheme.typography.labelLarge,
                    fontWeight = FontWeight.SemiBold,
                    color = ink.copy(alpha = 0.85f),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Spacer(Modifier.width(Space.sm))
                KindChip(course.isLab, fill = card)
            }
            Row(Modifier.padding(top = 6.dp), verticalAlignment = Alignment.Bottom) {
                Text(
                    buildAnnotatedString {
                        append(course.percentLabel.removeSuffix("%"))
                        if (course.total > 0) withStyle(SpanStyle(fontSize = 36.sp, fontWeight = FontWeight.Bold, letterSpacing = (-0.7).sp)) { append("%") }
                    },
                    style = MaterialTheme.typography.displayLargeEmphasized.nums(fontSize = 76.sp, lineHeight = 72.sp, letterSpacing = (-4).sp),
                    color = ink,
                    maxLines = 1,
                )
                Column(Modifier.weight(1f).padding(start = Space.md, bottom = Space.sm), horizontalAlignment = Alignment.End) {
                    (course.debarLine() ?: course.advice(showSkip = true))?.let {
                        Text(it, style = MaterialTheme.typography.titleSmallEmphasized.nums(fontWeight = FontWeight.Bold), color = ink, textAlign = TextAlign.End)
                    }
                    Text(standing.label, style = MaterialTheme.typography.labelMedium, color = ink.copy(alpha = 0.8f), textAlign = TextAlign.End)
                }
            }
            AttendanceLine(course.percent, tone.base, Modifier.padding(top = 18.dp), thickness = 14.dp, track = ink.copy(alpha = 0.14f), halo = tone.subtle)
            // "75%" centred under the tick.
            Text(
                "75%",
                Modifier.fillMaxWidth().padding(top = 2.dp, bottom = Space.lg).layout { m, c ->
                    val p = m.measure(c.copy(minWidth = 0))
                    layout(c.maxWidth, p.height) { p.place((c.maxWidth * 0.75f - p.width / 2f).roundToInt(), 0) }
                },
                style = MaterialTheme.typography.labelSmall,
                fontWeight = FontWeight.SemiBold,
                color = ink.copy(alpha = 0.75f),
            )
        }
        val (attended, held) = course.sessions
        val stats = listOf("$attended" to "Attended", "${held - attended}" to "Missed", "$held" to if (course.isLab) "Sessions" else "Classes")
        Row(Modifier.fillMaxWidth().testTag("attendance.detail.stats"), horizontalArrangement = Arrangement.spacedBy(2.dp)) {
            stats.forEachIndexed { i, (value, label) ->
                Column(Modifier.weight(1f).background(card, connected(i, stats.size, 22.dp, 8.dp)).padding(horizontal = Space.md, vertical = 10.dp)) {
                    Text(value, style = MaterialTheme.typography.headlineSmallEmphasized.nums(fontSize = 24.sp, fontWeight = FontWeight.W800), color = MaterialTheme.colorScheme.onSurface, maxLines = 1)
                    Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1)
                }
            }
        }
    }
}
