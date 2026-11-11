package vitapmate

import vitapmate.ui.OpenHaptic
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import vitapmate.ui.Radii
import com.composables.icons.lucide.ChevronRight
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.SheetValue
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import vitapmate.data.epochDayOf
import vitapmate.data.minuteOf
import vitapmate.observability.ActivityHistory
import kotlinx.coroutines.withContext
import kotlinx.coroutines.Dispatchers
import androidx.compose.runtime.produceState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.CircleMinus
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Mail
import com.composables.icons.lucide.SquareActivity
import com.composables.icons.lucide.X
import kotlinx.coroutines.flow.StateFlow
import vitapmate.data.LocalInspector
import vitapmate.data.OtpPrompt
import vitapmate.observability.Inspector
import vitapmate.observability.Run
import vitapmate.observability.RunStep
import vitapmate.observability.RunTrigger
import vitapmate.observability.StepState
import vitapmate.observability.settleOtp
import vitapmate.ui.AppTheme
import vitapmate.ui.EmptyState
import vitapmate.ui.Tone
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.is24Hour
import vitapmate.ui.StatusChip
import vitapmate.ui.neutralTone
import vitapmate.ui.nums
import vitapmate.ui.rememberNow

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
        logged?.let { live -> (live + saved.filter { s -> live.none { it.id == s.id } }).sortedByDescending { it.startT } }
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
            Text(
                g,
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
    val end = run.endT ?: run.startT
    val day = epochDayOf(end)
    return when (val days = epochDayOf(now) - day) {
        0 -> if (now - end < 15 * 60_000) "Now" else "Earlier today"
        1 -> "Yesterday"
        in 2..6 -> formatDay(day, "EEEE")
        else -> formatDay(day, "EEE d MMM")
    }
}

/** One run: a header row (state, title, what set it off and when) over its one-line step chain; tapped, each step's time and pages. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun RunCard(run: Run, container: Color, shape: Shape, modifier: Modifier) {
    var open by rememberSaveable(run.id) { mutableStateOf(false) }
    val now by rememberNow(if (run.running) 1_000 else 30_000)
    Surface({ open = !open }, modifier.fillMaxWidth().testTag("activity.${run.id}"), shape = shape, color = if (run.running) AppTheme.tones.accentTone.subtle else container) {
        Column(Modifier.animateContentSize(MaterialTheme.motionScheme.defaultSpatialSpec()).padding(14.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                RunGlyph(run)
                Text(run.title, Modifier.weight(1f), style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Text(
                    "${triggerText(run.trigger)} · ${whenText(run.startT, now)}",
                    style = MaterialTheme.typography.labelSmall.nums(),
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                )
            }
            StepLine(run.steps, Modifier.padding(start = 50.dp, top = 10.dp))
            RouteLine(run.route, run.fallback, Modifier.padding(start = 50.dp, top = 8.dp))
            AnimatedVisibility(
                open,
                enter = expandVertically(MaterialTheme.motionScheme.defaultSpatialSpec()) + fadeIn(MaterialTheme.motionScheme.defaultEffectsSpec()),
                exit = shrinkVertically(MaterialTheme.motionScheme.fastSpatialSpec()) + fadeOut(MaterialTheme.motionScheme.fastEffectsSpec()),
            ) {
                StepDetails(run, now)
            }
        }
    }
}

/** The run's state in 38 dp: a loading indicator while it runs, else a tinted cookie (a circle when neutral). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun RunGlyph(run: Run) {
    val pop = MaterialTheme.motionScheme.fastSpatialSpec<Float>()
    AnimatedContent(
        run.running,
        transitionSpec = { (scaleIn(pop) + fadeIn()) togetherWith (scaleOut() + fadeOut()) },
        label = "run-glyph",
    ) { running ->
        if (running) LoadingIndicator(Modifier.size(38.dp))
        else {
            val look = lookOf(
                when {
                    run.failed -> StepState.Failed
                    run.waiting -> StepState.WaitingOtp
                    run.steps.all { it.state == StepState.Skipped } -> StepState.Skipped
                    else -> StepState.Done
                },
            )
            val neutral = look.tone == neutralTone
            Box(Modifier.size(38.dp).background(look.tone.subtle, if (neutral) CircleShape else MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                Icon(look.icon, null, Modifier.size(18.dp), tint = if (neutral) look.tone.onSubtle else look.tone.base)
            }
        }
    }
}

/** How VTOP's answers came ("Proxy") as a chip, and "Server failed → Direct" after a fallback. */
@Composable
internal fun RouteLine(route: String?, fallback: String?, modifier: Modifier = Modifier) {
    if (route == null && fallback == null) return
    Row(modifier, verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        route?.let { StatusChip(it, neutralTone, Modifier.testTag("route.chip")) }
        fallback?.let { Text(it, style = MaterialTheme.typography.labelSmall, color = AppTheme.tones.warning.base, maxLines = 1, overflow = TextOverflow.Ellipsis) }
    }
}

/** Each step as a chip ("• Timetable (not signed in)") with a chevron between, on one line clipped at the end. */
@Composable
private fun StepLine(steps: List<RunStep>, modifier: Modifier) {
    val scheme = MaterialTheme.colorScheme
    val danger = AppTheme.tones.danger.base
    Box(modifier.fillMaxWidth().clipToBounds()) {
        Row(Modifier.wrapContentWidth(Alignment.Start, unbounded = true), horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
            steps.forEachIndexed { i, s ->
                if (i > 0) Icon(Lucide.ChevronRight, null, Modifier.size(13.dp), tint = scheme.outline)
                val muted = s.state == StepState.Skipped || s.state == StepState.Cached || s.state == StepState.WaitingOtp
                Row(
                    Modifier.height(25.dp).background(scheme.surfaceContainerHigh, CircleShape).padding(horizontal = 10.dp),
                    horizontalArrangement = Arrangement.spacedBy(6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Box(Modifier.size(7.dp)) { Dot(dotColor(s.state), s.state == StepState.Running) }
                    Text(
                        buildAnnotatedString {
                            withStyle(SpanStyle(color = if (muted) scheme.onSurfaceVariant else scheme.onSurface)) { append(s.label + if (s.count > 1) " ×${s.count}" else "") }
                            noteOf(s)?.let { withStyle(SpanStyle(color = if (s.state == StepState.Failed) danger else scheme.onSurfaceVariant)) { append(" ($it)") } }
                        },
                        style = MaterialTheme.typography.labelMedium,
                        maxLines = 1,
                        softWrap = false,
                    )
                }
            }
        }
    }
}

/** A step's dot; a running one breathes. */
@Composable
private fun Dot(color: Color, running: Boolean) {
    val alpha = if (running) {
        rememberInfiniteTransition(label = "dot").animateFloat(1f, 0.3f, infiniteRepeatable(tween(700), RepeatMode.Reverse), label = "dot-alpha").value
    } else 1f
    Box(Modifier.fillMaxSize().alpha(alpha).background(color, CircleShape))
}

@Composable
private fun dotColor(state: StepState): Color {
    val tones = AppTheme.tones
    return when (state) {
        StepState.Done -> tones.success.base
        StepState.Running -> tones.accent
        StepState.Failed -> tones.danger.base
        // Waiting for the OTP is part of signing in: neutral, like a skipped or cached step.
        StepState.WaitingOtp, StepState.Skipped, StepState.Cached -> MaterialTheme.colorScheme.outline
    }
}

/** The few words after a step's name in the chain. */
private fun noteOf(s: RunStep): String? = when (s.state) {
    StepState.Failed, StepState.Skipped -> s.reason?.lowercase()
    StepState.WaitingOtp -> s.reason?.lowercase() ?: "waiting"
    StepState.Cached -> "saved copy"
    StepState.Running, StepState.Done -> null
}

/** Each step with its time, and the VTOP pages it fetched. */
@Composable
private fun StepDetails(run: Run, now: Long) {
    val inspector = LocalInspector.current
    val scheme = MaterialTheme.colorScheme
    val rail = scheme.outlineVariant
    Column(Modifier.padding(start = 45.dp, top = 14.dp)) {
        // The steps on a rail: a 2 dp line through their dots.
        Column(
            Modifier.drawBehind { drawLine(rail, Offset(3.5.dp.toPx(), 0f), Offset(3.5.dp.toPx(), size.height), 2.dp.toPx()) },
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            run.steps.forEach { step ->
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Box(Modifier.size(7.dp).background(dotColor(step.state), CircleShape))
                        Spacer(Modifier.width(8.dp))
                        Text(step.label + if (step.count > 1) " ×${step.count}" else "", Modifier.weight(1f), style = MaterialTheme.typography.labelLarge)
                        Spacer(Modifier.width(8.dp))
                        Text(duration((step.endT ?: now) - step.startT), style = MaterialTheme.typography.labelSmall.nums(), color = scheme.onSurfaceVariant)
                    }
                    (step.reason ?: stateText(step.state))?.let {
                        Text(it, Modifier.padding(start = 15.dp), style = MaterialTheme.typography.labelSmall, color = if (step.state == StepState.Failed) AppTheme.tones.danger.base else scheme.onSurfaceVariant)
                    }
                    step.pages.forEach { hit ->
                        Row(Modifier.padding(start = 15.dp), verticalAlignment = Alignment.CenterVertically) {
                            Text(hit.name, Modifier.weight(1f), style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            Text(
                                listOfNotNull(hit.status.takeUnless { it.startsWith("2") || it.startsWith("3") }, hit.ms?.let(::duration)).joinToString(" · "),
                                style = MaterialTheme.typography.labelSmall.nums(),
                                color = scheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        }
        FilledTonalButton(
            { inspector.open(run.id) },
            Modifier.align(Alignment.End).padding(top = 12.dp).height(36.dp).testTag("activity.log"),
            contentPadding = PaddingValues(horizontal = 16.dp),
        ) {
            Icon(Lucide.SquareActivity, null, Modifier.size(15.dp))
            Spacer(Modifier.width(7.dp))
            Text("Log", style = MaterialTheme.typography.labelLarge)
        }
    }
}

private class Look(val icon: ImageVector, val tone: Tone)

@Composable
private fun lookOf(state: StepState): Look {
    val tones = AppTheme.tones
    return when (state) {
        StepState.Done -> Look(Lucide.Check, tones.success)
        StepState.Running -> Look(Lucide.Check, tones.accentTone)
        // Part of signing in, not a failure.
        StepState.WaitingOtp -> Look(Lucide.Mail, neutralTone)
        StepState.Failed -> Look(Lucide.X, tones.danger)
        StepState.Skipped -> Look(Lucide.CircleMinus, neutralTone)
        StepState.Cached -> Look(Lucide.History, neutralTone)
    }
}

private fun stateText(state: StepState): String? = when (state) {
    StepState.Cached -> "Served the saved copy"
    StepState.WaitingOtp -> "Waiting for the OTP"
    else -> null
}

private fun triggerText(trigger: RunTrigger) = when (trigger) {
    RunTrigger.Opened -> "Opened"
    RunTrigger.Pulled -> "Pulled"
    RunTrigger.Retried -> "Retried"
    RunTrigger.Automatic -> "Auto"
    RunTrigger.Returned -> "Resumed"
    RunTrigger.AfterOtp -> "After OTP"
    RunTrigger.SignedIn -> "Signed in"
    RunTrigger.Tapped -> "Tapped"
}

/** "now", "5 min" within the hour, else the clock time. */
@Composable
private fun whenText(t: Long, now: Long): String {
    val ms = now - t
    return when {
        ms < 60_000 -> "now"
        ms < 3_600_000 -> "${ms / 60_000} min"
        else -> clock(minuteOf(t), is24Hour())
    }
}

/** 640 → "640 ms", 1240 → "1.2 s", 65000 → "1m 5s". */
private fun duration(ms: Long): String = when {
    ms < 0 -> "0 ms"
    ms < 1_000 -> "$ms ms"
    ms < 60_000 -> "%.1f s".format(ms / 1000.0)
    else -> "${ms / 60_000}m ${ms % 60_000 / 1000}s"
}
