package vitapmate.ui

import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.SelectableDates
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import vitapmate.data.DAY_MS

/**
 * A day in [range] (epoch days) from the M3 date picker. Picking a day picks it and closes, unless
 * [setButton] is given: then a Set button with that modifier does.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DayPicker(day: Int, range: IntRange, onDismiss: () -> Unit, modifier: Modifier = Modifier, setButton: Modifier? = null, onPick: (Int) -> Unit) {
    val state = rememberDatePickerState(
        initialSelectedDateMillis = day.coerceIn(range) * DAY_MS,
        yearRange = formatDay(range.first, "yyyy").toInt()..formatDay(range.last, "yyyy").toInt(),
        selectableDates = object : SelectableDates {
            override fun isSelectableDate(utcTimeMillis: Long) = (utcTimeMillis / DAY_MS).toInt() in range
        },
    )
    if (setButton == null) {
        LaunchedEffect(state) {
            snapshotFlow { state.selectedDateMillis }.collect { ms ->
                if (ms != null && (ms / DAY_MS).toInt() != day) onPick((ms / DAY_MS).toInt())
            }
        }
    }
    DatePickerDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            if (setButton != null) {
                TextButton({ state.selectedDateMillis?.let { onPick((it / DAY_MS).toInt()) } ?: onDismiss() }, setButton) { Text("Set") }
            }
        },
        modifier = modifier,
        dismissButton = { TextButton(onDismiss) { Text("Cancel") } },
    ) { DatePicker(state, showModeToggle = false) }
}
