package vitapmate.features.docs

import vitapmate.ui.OpenHaptic
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3Api
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
@OptIn(ExperimentalMaterial3Api::class, ExperimentalComposeUiApi::class)
@Composable
fun MessTimingsSheet(onDismiss: () -> Unit) {
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    val scope = rememberCoroutineScope()
    val h24 = is24Hour()
    var picking by remember { mutableStateOf<Pair<Meal, Boolean>?>(null) }
    val windows = MessTimings.windows
    fun log(what: String) = scope.logAction("settings.mess_timings", "ui.settings", "mess timings $what")

    OpenHaptic()
    ModalBottomSheet(onDismiss, Modifier.semantics { testTagsAsResourceId = true }.testTag("settings.mess_timings.sheet"), sheet, containerColor = MaterialTheme.colorScheme.surface) {
        CompositionLocalProvider(LocalContentPadding provides PaddingValues()) {
            Column(Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = Space.xl).padding(bottom = Space.md)) {
                Text("Mess Timings", Modifier.padding(bottom = Space.lg), style = MaterialTheme.typography.headlineSmallEmphasized)
                Meal.entries.forEach { meal ->
                    Row(Modifier.padding(bottom = Space.sm + 2.dp), verticalAlignment = Alignment.CenterVertically) {
                        Text(meal.label, Modifier.weight(1f), style = MaterialTheme.typography.bodyLarge)
                        TimeButton(clock(windows.getValue(meal).start, h24), "settings.mess_timings.${meal.name.lowercase()}.start") { picking = meal to true }
                        Text("–", Modifier.padding(horizontal = Space.sm), color = MaterialTheme.colorScheme.onSurfaceVariant)
                        TimeButton(clock(windows.getValue(meal).end, h24), "settings.mess_timings.${meal.name.lowercase()}.end") { picking = meal to false }
                    }
                }
                OutlinedButton({ MessTimings.reset(); log("reset") }, Modifier.fillMaxWidth().padding(top = Space.sm).testTag("settings.mess_timings.reset")) {
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

@Composable
private fun TimeButton(label: String, tag: String, onClick: () -> Unit) {
    OutlinedButton(onClick, Modifier.width(112.dp).testTag(tag), contentPadding = PaddingValues(horizontal = Space.sm)) {
        Text(label, style = MaterialTheme.typography.bodyMedium.nums())
    }
}
