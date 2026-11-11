package vitapmate.features.calendar

import vitapmate.ui.UpdatedFooter
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarX
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch
import vitapmate.core.AcademicCalendar
import vitapmate.core.CalendarDay
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.EmptyView
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.PageScaffold
import vitapmate.ui.Radii
import vitapmate.ui.ScrollToTopOnReselect
import vitapmate.ui.SectionHeader
import vitapmate.ui.Skeleton
import vitapmate.ui.Space
import vitapmate.ui.formatDay

/**
 * The academic calendar (`page.calendar`, log src `ui.calendar`, flows `calendar.open`,
 * `calendar.refresh`, `calendar.month`, `calendar.day`; state dump key `calendar`): what is left
 * of the semester, a month grid to swipe through, and the shown month's events.
 */
@Composable
fun CalendarScreen() {
    val page = rememberPage("calendar", { c, f -> c.cachedCalendar(f) }, { c, f -> c.fetchCalendar(f) }, { it.updatedAt }) { it.months.size }
    PageScaffold(
        page.state,
        { page.refresh("pull") },
        isEmpty = { it.months.isEmpty() },
        empty = {
            EmptyView(
                Lucide.CalendarX,
                "No calendar yet",
                Modifier.testTag("calendar.empty"),
                message = "VTOP hasn't published this semester's calendar. Pull down to check again.",
            )
        },
        loading = { CalendarSkeleton() },
    ) {
        // Its own state per semester: the shown month and day are saved (and reused when the
        // content outlives a semester switch), so another semester would open on this one's page.
        key(it.semesterId) { CalendarContent(it) }
    }
}

@Composable
private fun CalendarContent(cal: AcademicCalendar) {
    val months = cal.months
    // The month holding today; before the semester its first month, after it its last.
    val initial = remember(cal) {
        months.indexOfFirst { m -> cal.today in m.days.first().day..m.days.last().day }
            .takeIf { it >= 0 } ?: if (cal.today < months.first().days.first().day) 0 else months.lastIndex
    }
    val pager = rememberPagerState(initial) { months.size }
    var selected by rememberSaveable { mutableStateOf<Int?>(null) }
    // How the next page change came about: a swipe unless a button set it.
    var how by remember { mutableStateOf("swipe") }
    val scope = rememberCoroutineScope()
    val month = months[pager.currentPage.coerceIn(months.indices)]

    LaunchedEffect(pager, months) {
        snapshotFlow { pager.settledPage }.drop(1).collect { i ->
            val m = months.getOrNull(i) ?: return@collect
            flow("calendar.month") { id ->
                Log.info("ui.calendar", "month ${m.month}", id, mapOf("month" to m.month, "how" to how, "events" to "${m.events.size}"))
            }
            how = "swipe"
        }
    }
    LaunchedEffect(month, selected) {
        StateRegistry.report(
            "calendar",
            mapOf(
                "semester" to cal.semesterId,
                "month" to month.month,
                "months" to months.size,
                "events" to month.events.size,
                "daysLeft" to cal.daysLeft,
                "selectedDay" to selected?.let { formatDay(it, "yyyy-MM-dd") },
            ),
        )
    }
    DisposableEffect(Unit) { onDispose { StateRegistry.remove("calendar") } }

    fun step(delta: Int) {
        val target = (pager.currentPage + delta).coerceIn(months.indices)
        if (target == pager.currentPage) return
        how = "tap"
        scope.launch { pager.animateScrollToPage(target) }
    }
    fun tapDay(d: CalendarDay) {
        selected = if (selected == d.day) null else d.day
        scope.launch {
            flow("calendar.day") { id ->
                Log.info("ui.calendar", "day ${d.date}", id, mapOf("date" to d.date, "mark" to (d.mark?.name ?: "none"), "selected" to "${selected != null}"))
            }
        }
    }

    val list = rememberLazyListState()
    ScrollToTopOnReselect(list)
    LazyColumn(
        Modifier.fillMaxSize().testTag("calendar.list"),
        list,
        contentPadding = PaddingValues(
            start = Space.sm,
            top = Space.xs,
            end = Space.sm,
            bottom = LocalContentPadding.current.calculateBottomPadding() + Space.lg,
        ),
    ) {
        item("outlook") { Outlook(cal, Modifier.padding(horizontal = Space.xs, vertical = Space.sm)) }
        item("month") { MonthCard(months, pager, cal.today, selected, ::tapDay, ::step) }
        item("events") {
            val effects = MaterialTheme.motionScheme.defaultEffectsSpec<Float>()
            AnimatedContent(month, Modifier.fillMaxWidth(), transitionSpec = { fadeIn(effects) togetherWith fadeOut(effects) }, contentKey = { it.month }) { m ->
                Column {
                    SectionHeader(formatDay(m.days.first().day, "MMMM"))
                    MonthEvents(m, cal.today, Modifier.testTag("calendar.events"))
                }
            }
        }
        item("updated") { UpdatedFooter() }
    }
}

@Composable
private fun CalendarSkeleton() {
    Column(Modifier.fillMaxSize().padding(Space.sm), verticalArrangement = Arrangement.spacedBy(Space.md)) {
        Skeleton(Modifier.fillMaxWidth(), height = 96.dp, radius = Radii.xl)
        Skeleton(Modifier.fillMaxWidth(), height = 360.dp, radius = Radii.lg)
        repeat(4) { Skeleton(Modifier.fillMaxWidth(), height = 76.dp) }
    }
}
