package vitapmate

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
import com.composables.icons.lucide.ChevronRight
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.draw.drawBehind
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.snapshotFlow
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import vitapmate.data.minuteOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.CircleMinus
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Mail
import com.composables.icons.lucide.SquareActivity
import com.composables.icons.lucide.X
import vitapmate.data.LocalInspector
import vitapmate.observability.Run
import vitapmate.observability.RunStep
import vitapmate.observability.RunTrigger
import vitapmate.observability.StepState
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone
import vitapmate.ui.clock
import vitapmate.ui.is24Hour
import vitapmate.ui.StatusChip
import vitapmate.ui.neutralTone
import vitapmate.ui.nums
import vitapmate.ui.rememberNow

/** One run: a header row (state, title, what set it off and when) over its one-line step chain; tapped, each step's time and pages. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun RunCard(run: Run, container: Color, shape: Shape, modifier: Modifier) {
    var open by rememberSaveable(run.id) { mutableStateOf(false) }
    val now by rememberNow(if (run.running) 1_000 else 30_000)
    Surface({ open = !open }, modifier.fillMaxWidth().testTag("activity.${run.id}"), shape = shape, color = if (run.running) AppTheme.tones.accentTone.subtle else if (run.failed) AppTheme.tones.danger.subtle else container) {
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
            StepLine(run.steps, run.running || run.failed, Modifier.padding(start = 50.dp, top = 10.dp))
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

/**
 * Each step as a chip ("• Timetable (not signed in)") with a chevron between, on one line; when
 * it doesn't fit it shows the start, or with [latest] (running or failed) the last steps.
 */
@Composable
private fun StepLine(steps: List<RunStep>, latest: Boolean, modifier: Modifier) {
    val scheme = MaterialTheme.colorScheme
    val danger = AppTheme.tones.danger.base
    val scroll = rememberScrollState()
    // Follows the chain as steps are added (the width is known only after layout).
    LaunchedEffect(latest) { if (latest) snapshotFlow { scroll.maxValue }.collect { scroll.animateScrollTo(it) } else scroll.scrollTo(0) }
    Box(modifier.fillMaxWidth()) {
        Row(Modifier.horizontalScroll(scroll, enabled = false), horizontalArrangement = Arrangement.spacedBy(4.dp), verticalAlignment = Alignment.CenterVertically) {
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
    // Waiting for a lane.
    StepState.Running -> s.reason?.let { "waiting" }
    StepState.Done -> null
}

/** Each step with its time, and the VTOP pages it fetched. */
@Composable
private fun StepDetails(run: Run, now: Long) {
    val inspector = LocalInspector.current
    val scheme = MaterialTheme.colorScheme
    val rail = scheme.outlineVariant
    // In an inset container, so the steps read as one block under the chain.
    Column(Modifier.padding(start = 45.dp, top = 12.dp).fillMaxWidth().background(scheme.surfaceContainerHighest, RoundedCornerShape(18.dp)).padding(12.dp)) {
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
