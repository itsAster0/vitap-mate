package vitapmate.features.docs

import vitapmate.ui.OpenHaptic
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.ui.draw.clip
import com.composables.icons.lucide.Coffee
import com.composables.icons.lucide.Cookie
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Moon
import com.composables.icons.lucide.Soup
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimePicker
import androidx.compose.material3.TimePickerDialog
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.material3.rememberTimePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import vitapmate.data.Meal
import vitapmate.data.MealWindow
import vitapmate.data.MessTimings
import vitapmate.observability.Log
import vitapmate.observability.logAction
import vitapmate.ui.LocalContentPadding
import vitapmate.ui.Space
import vitapmate.ui.clock
import vitapmate.ui.is24Hour
import vitapmate.ui.nums
import vitapmate.ui.showToast

/** Start of each meal, breakfast to dinner: "7:15 · 12:30 · 4:30 · 7:15" (Settings' line). */
@Composable
fun messTimingsSummary(): String {
    val h24 = is24Hour()
    return Meal.entries.joinToString(" · ") { clock(MessTimings.windows.getValue(it).start, h24).substringBefore(' ') }
}

/** Settings → Mess Timings: each meal's serving hours, which mark the meal on now in the mess menu. */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalComposeUiApi::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun MessTimingsSheet(onDismiss: () -> Unit) {
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    val scope = rememberCoroutineScope()
    val h24 = is24Hour()
    var picking by remember { mutableStateOf<Pair<Meal, Boolean>?>(null) }
    val windows = MessTimings.windows
    val scheme = MaterialTheme.colorScheme
    fun log(what: String) = scope.logAction("settings.mess_timings", "ui.settings", "mess timings $what")

    OpenHaptic()
    ModalBottomSheet(onDismiss, Modifier.semantics { testTagsAsResourceId = true }.testTag("settings.mess_timings.sheet"), sheet, containerColor = MaterialTheme.colorScheme.surface) {
        CompositionLocalProvider(LocalContentPadding provides PaddingValues()) {
            Column(Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = Space.xl).padding(bottom = Space.md)) {
                Text("Mess Timings", Modifier.padding(bottom = Space.lg), style = MaterialTheme.typography.headlineSmallEmphasized)
                // One joined block: each meal with its icon and its start – end as a joined pair.
                Column(Modifier.clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                    Meal.entries.forEach { meal ->
                        Row(
                            Modifier.fillMaxWidth().clip(ListItemDefaults.segmentedShapes(1, 3).shape).background(scheme.surfaceContainerHigh).padding(horizontal = Space.md, vertical = Space.sm + 2.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            val icon = when (meal) {
                                Meal.Breakfast -> Lucide.Coffee
                                Meal.Lunch -> Lucide.Soup
                                Meal.Snacks -> Lucide.Cookie
                                Meal.Dinner -> Lucide.Moon
                            }
                            Box(Modifier.size(40.dp).background(scheme.secondaryContainer, CircleShape), Alignment.Center) {
                                Icon(icon, null, Modifier.size(20.dp), tint = scheme.onSecondaryContainer)
                            }
                            Text(meal.label, Modifier.weight(1f).padding(horizontal = Space.md), style = MaterialTheme.typography.bodyLarge)
                            Row(horizontalArrangement = Arrangement.spacedBy(2.dp)) {
                                TimeButton(clock(windows.getValue(meal).start, h24), "settings.mess_timings.${meal.name.lowercase()}.start", start = true) { picking = meal to true }
                                TimeButton(clock(windows.getValue(meal).end, h24), "settings.mess_timings.${meal.name.lowercase()}.end", start = false) { picking = meal to false }
                            }
                        }
                    }
                }
                OutlinedButton({ MessTimings.reset(); log("reset") }, Modifier.fillMaxWidth().padding(top = Space.lg).testTag("settings.mess_timings.reset")) {
                    Text("Reset to VIT-AP timings")
                }
            }
        }
    }
    picking?.let { (meal, start) ->
        val current = windows.getValue(meal)
        val minute = if (start) current.start else current.end
        val state = rememberTimePickerState(minute / 60, minute % 60, h24)
        TimePickerDialog(
            onDismissRequest = { picking = null },
            confirmButton = {
                TextButton({
                    picking = null
                    val picked = state.hour * 60 + state.minute
                    val next = if (start) MealWindow(picked, current.end) else MealWindow(current.start, picked)
                    if (next.end > next.start) {
                        MessTimings.windows = windows + (meal to next)
                        log("${meal.name} ${if (start) "start" else "end"}")
                    } else {
                        showToast("Check the times", "${meal.label} has to end after it starts.")
                    }
                }, Modifier.testTag("settings.mess_timings.ok")) { Text("Save") }
            },
            title = { Text("${meal.label} ${if (start) "starts" else "ends"}") },
            dismissButton = { TextButton({ picking = null }) { Text("Cancel") } },
        ) { TimePicker(state) }
    }
}

/** One end of a meal's joined start – end pair: the [start] one rounds its outer left side, the end one its right. */
@Composable
private fun TimeButton(label: String, tag: String, start: Boolean, onClick: () -> Unit) {
    val outer = 20.dp
    val inner = 6.dp
    FilledTonalButton(
        onClick,
        Modifier.width(96.dp).testTag(tag),
        shape = if (start) RoundedCornerShape(outer, inner, inner, outer) else RoundedCornerShape(inner, outer, outer, inner),
        colors = ButtonDefaults.filledTonalButtonColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLowest),
        contentPadding = PaddingValues(horizontal = Space.xs),
    ) {
        Text(label, style = MaterialTheme.typography.labelLarge.nums(), maxLines = 1)
    }
}
