package vitapmate

import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.CalendarMinus
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sparkles
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import vitapmate.data.Settings
import vitapmate.features.attendance.SKIP_DISCLAIMER
import vitapmate.observability.Log
import vitapmate.ui.AppDialog
import vitapmate.ui.rememberHaptics

/** Wait before the calculated-insights disclaimer can be accepted, in seconds. */
private const val INSIGHTS_DISCLAIMER_SECONDS = 5

/** The disclaimer shown before "Calculated insights" turns on. */
private const val INSIGHTS_DISCLAIMER =
    "Totals, GPAs, attendance projections, cross-page views and calculators are computed by the app " +
        "from VTOP data. They can be inaccurate or out of date. Always cross-verify important " +
        "numbers directly on VTOP. The app isn't responsible for any issues caused by relying on " +
        "these numbers."

/**
 * Settings and Onboarding's "Insights" group: Calculated insights (with a 5 s timed disclaimer),
 * the dependent "Classes you can skip" switch, and (when [onGuide] is given) the attendance guide.
 */
@Composable
fun InsightsSetting(onGuide: (() -> Unit)? = null) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    var insightsDisclaimer by remember { mutableStateOf(false) }
    var skipDisclaimer by remember { mutableStateOf(false) }
    val refreshSurfaces = {
        scope.launch {
            Widgets.refresh(context)
            LiveUpdates.refresh(context, "settings.insights")
            ClassMode.refresh(context, "classmode.insights")
            CalendarSync.sync(context, "calendar.insights")
        }
    }
    // Turning calculated insights on asks first (a timed disclaimer); turning it off doesn't.
    val setInsights = { on: Boolean ->
        if (on) {
            insightsDisclaimer = true
        } else {
            Settings.calculatedInsights = false
            haptics.toggle(false)
            Log.info("ui.settings", "calculated insights off")
            refreshSurfaces()
        }
    }
    // Turning skip advice on asks first (a disclaimer); turning it off doesn't.
    val setSkip = { on: Boolean ->
        if (on) {
            skipDisclaimer = true
        } else {
            Settings.showSkipAdvice = false
            haptics.toggle(false)
            Log.info("ui.settings", "skip advice off")
            refreshSurfaces()
        }
    }

    val insights = Settings.calculatedInsights
    val skipOn = Settings.effectiveSkipAdvice
    ListGroup("Insights") {
        GroupItem(
            "Calculated insights",
            "settings.calculated_insights",
            Lucide.Sparkles,
            "Totals, GPAs, projections, and calculators derived from VTOP",
            onClick = { setInsights(!insights) },
            trailing = { ItemSwitch(insights, { setInsights(it) }) },
            on = insights,
        )
        GroupItem(
            "Classes you can skip",
            "settings.skip_advice",
            Lucide.CalendarMinus,
            if (insights) "On attendance and the timetable" else "Turn on Calculated insights first",
            enabled = insights,
            onClick = if (insights) ({ setSkip(!Settings.showSkipAdvice) }) else null,
            trailing = { ItemSwitch(skipOn, { setSkip(it) }, enabled = insights) },
            on = skipOn,
        )
        if (insights && onGuide != null) {
            GroupItem(
                "How the numbers work",
                "settings.attendance_guide",
                Lucide.BookOpen,
                "What each number means and when it can be off",
                onClick = onGuide,
                trailing = { Chevron() },
            )
        }
    }

    if (insightsDisclaimer) {
        var secondsLeft by remember { mutableIntStateOf(INSIGHTS_DISCLAIMER_SECONDS) }
        LaunchedEffect(Unit) {
            while (secondsLeft > 0) {
                delay(1_000L)
                secondsLeft--
            }
        }
        AppDialog(
            { insightsDisclaimer = false },
            "Calculated insights",
            if (secondsLeft > 0) "Accept ($secondsLeft)" else "Accept",
            {
                insightsDisclaimer = false
                Settings.calculatedInsights = true
                haptics.toggle(true)
                Log.info("ui.settings", "calculated insights on (disclaimer accepted)")
                refreshSurfaces()
            },
            icon = Lucide.Sparkles,
            confirmEnabled = secondsLeft == 0,
            confirmTag = "settings.calculated_insights.accept",
            dismissTag = "settings.calculated_insights.cancel",
        ) { Text(INSIGHTS_DISCLAIMER) }
    }

    if (skipDisclaimer) {
        AppDialog(
            { skipDisclaimer = false },
            "Classes you can skip",
            "Turn on",
            {
                skipDisclaimer = false
                Settings.showSkipAdvice = true
                haptics.toggle(true)
                Log.info("ui.settings", "skip advice on (disclaimer accepted)")
                refreshSurfaces()
            },
            icon = Lucide.CalendarMinus,
            confirmTag = "settings.skip_disclaimer.on",
            dismissTag = "settings.skip_disclaimer.cancel",
        ) { Text(SKIP_DISCLAIMER) }
    }
}
