package vitapmate

import vitapmate.ui.OpenHaptic
import androidx.compose.foundation.background
import vitapmate.ui.Radii
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.SheetValue
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import vitapmate.data.epochDayOf
import vitapmate.observability.ActivityHistory
import kotlinx.coroutines.withContext
import kotlinx.coroutines.Dispatchers
import androidx.compose.runtime.produceState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.SquareActivity
import kotlinx.coroutines.flow.StateFlow
import vitapmate.data.OtpPrompt
import vitapmate.observability.Inspector
import vitapmate.observability.Run
import vitapmate.observability.settleOtp
import vitapmate.ui.AppTheme
import vitapmate.ui.EmptyState
import vitapmate.ui.clock
import vitapmate.ui.formatDay

const val ACTIVITY = "activity"

/** [ActivityScreen] with the older saved history already shown (from Settings). */
const val ACTIVITY_HISTORY = "activity.history"


/** [source]'s runs with the OTP prompt's state, only this process's unless [older]; null until first read. */
@Composable
private fun rememberRuns(source: StateFlow<List<Run>?>, older: Boolean = false): List<Run>? {
    val runs by source.collectAsState()
    val pending = OtpPrompt.message != null
    return remember(runs, pending, older) {
        runs?.let { all -> if (older) all else all.filter { it.startT >= Inspector.startedAt } }?.settleOtp(pending, Inspector.startedAt)
    }
}

/**
 * What the app did on VTOP lately, the way the student lived it: what runs now, then the last
 * few runs, each a chain of steps ("Sign in › OTP › Timetable"). "See all" opens [ActivityScreen].
 * The Inspector (a run's "Log") keeps the technical lines.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ActivitySheet(onDismiss: () -> Unit) {
    // Opens once the runs are read, straight at its height (wrapping them, at most 4/5 of the
    // screen): no half-height stop, no jump when they arrive.
    val runs = rememberRuns(Inspector.recentRuns) ?: return
    val nav = LocalNav.current
    val state = rememberBottomSheetState(SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    val max = (LocalConfiguration.current.screenHeightDp * 0.8f).dp
    OpenHaptic()
    ModalBottomSheet(onDismiss, Modifier.testTag("activity.sheet"), sheetState = state, containerColor = MaterialTheme.colorScheme.surfaceContainerLow) {
        Row(Modifier.fillMaxWidth().padding(start = 20.dp, end = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text("Activity", Modifier.weight(1f), style = MaterialTheme.typography.titleMedium)
            TextButton({ onDismiss(); nav.push(Route.Screen(ACTIVITY)) }, Modifier.testTag("activity.all")) { Text("See all") }
        }
        val card = MaterialTheme.colorScheme.surfaceContainerHigh
        LazyColumn(Modifier.heightIn(max = max), contentPadding = PaddingValues(8.dp, 0.dp, 8.dp, 16.dp), verticalArrangement = Arrangement.spacedBy(RUN_GAP)) {
            runItems(runs, SHEET_RUNS, card)
        }
    }
}

/**
 * This app session's runs: Now, Earlier today, ...; "Show older" at the end (or [showOlder], from
 * Settings) adds every run the log files still hold from earlier sessions and the saved history
 * ([ActivityHistory]), grouped by day.
 */
@Composable
fun ActivityScreen(showOlder: Boolean = false) {
    var older by remember { mutableStateOf(showOlder) }
    val runs = rememberAllRuns(older)
    val card = AppTheme.colors.card
    Page("Activity", ACTIVITY) { padding ->
        LazyColumn(
            contentPadding = PaddingValues(start = 8.dp, end = 8.dp, top = padding.calculateTopPadding(), bottom = padding.calculateBottomPadding()),
            verticalArrangement = Arrangement.spacedBy(RUN_GAP),
        ) {
            runItems(runs, Int.MAX_VALUE, card)
            if (!older && runs != null) {
                item(key = "older") {
                    Box(Modifier.fillMaxWidth().animateItem().padding(vertical = 13.dp), Alignment.Center) {
                        FilledTonalButton({ older = true }, Modifier.testTag("activity.older")) { Text("Show older") }
                    }
                }
            }
        }
    }
}

/** This session's runs and, when [older], every earlier one the logs and [ActivityHistory] hold; newest first, null until read. */
@Composable
internal fun rememberAllRuns(older: Boolean): List<Run>? {
    val recent = rememberRuns(Inspector.recentRuns)
    // This session's runs stay while both files are read.
    val logged = (if (older) rememberRuns(Inspector.allRuns, older = true) else null) ?: recent
    // Older still: runs saved before the logs rolled over (ActivityHistory).
    val saved by produceState(emptyList<Run>(), older) { if (older) value = withContext(Dispatchers.IO) { ActivityHistory.load() } }
    return remember(logged, saved) {
        // Both newest first in the order they happened (not by clock time, which can jump); saved ones are older.
        logged?.let { live -> live + saved.filter { s -> live.none { it.id == s.id } } }
    }
}

private const val SHEET_RUNS = 6

/** The gap between run cards of a group; lists showing [runItems] space their items by it. */
internal val RUN_GAP = 3.dp

/**
 * Run cards in [container] colour under group headers (pills), each group's cards one segmented
 * block; at most [limit] finished ones; [inset] at the sides.
 */
internal fun LazyListScope.runItems(runs: List<Run>?, limit: Int, container: Color, inset: Dp = 0.dp, empty: String = "Nothing yet") {
    if (runs == null) return
    if (runs.isEmpty()) {
        item { EmptyState(Lucide.SquareActivity, empty, Modifier.fillMaxWidth().padding(vertical = 32.dp)) }
        return
    }
    val now = System.currentTimeMillis()
    val shown = runs.filter { it.running } + runs.filterNot { it.running }.take(limit)
    val groups = shown.groupBy { groupOf(it, now) }
    groups.entries.forEachIndexed { gi, (g, members) ->
        item(key = "group.$g", contentType = "group") {
            val failed = members.count { it.failed && !it.running }
            Text(
                buildAnnotatedString {
                    append(g)
                    // How many runs it holds, and how many failed ("Earlier today · 3 · 1 failed").
                    if (g != "Now") withStyle(SpanStyle(color = MaterialTheme.colorScheme.onSurfaceVariant)) {
                        append(" · ${members.size}")
                        if (failed > 0) append(" · $failed failed")
                    }
                },
                Modifier.animateItem().padding(start = inset + 6.dp, top = if (gi == 0) 2.dp else 13.dp, bottom = 5.dp)
                    .background(MaterialTheme.colorScheme.surfaceContainerHigh, CircleShape).padding(horizontal = 12.dp, vertical = 4.dp),
                style = MaterialTheme.typography.labelLarge,
                color = MaterialTheme.colorScheme.onSurface,
            )
        }
        members.forEachIndexed { i, run ->
            // The group's ends round to extraLarge; inner corners stay small.
            val shape = RoundedCornerShape(
                topStart = if (i == 0) Radii.xl else 6.dp,
                topEnd = if (i == 0) Radii.xl else 6.dp,
                bottomEnd = if (i == members.lastIndex) Radii.xl else 6.dp,
                bottomStart = if (i == members.lastIndex) Radii.xl else 6.dp,
            )
            item(key = run.id, contentType = "run") { RunCard(run, container, shape, Modifier.animateItem().padding(horizontal = inset)) }
        }
    }
}

private fun groupOf(run: Run, now: Long): String {
    if (run.running || run.waiting) return "Now"
    // A time ahead of the clock (it was set back since) counts as now.
    val end = minOf(run.endT ?: run.startT, now)
    val day = epochDayOf(end)
    return when (epochDayOf(now) - day) {
        0 -> if (now - end < 15 * 60_000) "Just now" else "Earlier today"
        1 -> "Yesterday"
        in 2..6 -> formatDay(day, "EEEE")
        else -> formatDay(day, "EEE d MMM")
    }
}
