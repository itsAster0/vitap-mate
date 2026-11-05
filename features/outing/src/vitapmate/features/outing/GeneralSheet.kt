package vitapmate.features.outing

import vitapmate.ui.rememberHaptics
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SelectableDates
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TimePicker
import androidx.compose.material3.TimePickerDialog
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.material3.rememberTimePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Calendar
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.Lucide
import vitapmate.core.GeneralOuting
import vitapmate.core.GeneralOutingForm
import vitapmate.core.OutingMoment
import vitapmate.core.OutingResult
import vitapmate.core.checkGeneralOuting
import vitapmate.core.generalOutingTimes
import vitapmate.core.generalReturnLastDay
import vitapmate.ui.Space
import vitapmate.ui.clock
import vitapmate.ui.formatDay
import vitapmate.ui.is24Hour
import vitapmate.ui.rememberNow

private const val DAY_MS = 86_400_000L

private enum class Picking { LeaveDate, LeaveTime, BackDate, BackTime }

/**
 * The general outing form: place, purpose, leaving and return (date and time, starting just over
 * a day ahead in the offered hours). Rules and texts from Rust ([checkGeneralOuting]); timing
 * problems show at once, text ones after the first Review. Review asks to confirm, then sends
 * once (flow `outing.apply`); [onSent] gets VTOP's yes, a no stays in the sheet.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
internal fun GeneralSheet(form: GeneralOuting, onDismiss: () -> Unit, onMaybeSent: () -> Unit, onSent: (OutingResult) -> Unit) {
    val haptics = rememberHaptics()
    val start = remember(form.rules) { generalOutingTimes(form.rules) }
    var place by rememberSaveable { mutableStateOf("") }
    var purpose by rememberSaveable { mutableStateOf("") }
    var leaveDay by rememberSaveable { mutableIntStateOf(start.leaving.day) }
    var leaveMinute by rememberSaveable { mutableIntStateOf(start.leaving.minute) }
    var backDay by rememberSaveable { mutableIntStateOf(start.returning.day) }
    var backMinute by rememberSaveable { mutableIntStateOf(start.returning.minute) }
    var showErrors by rememberSaveable { mutableStateOf(false) }
    var failure by rememberSaveable { mutableStateOf<String?>(null) }
    var sending by remember { mutableStateOf(false) }
    var confirming by remember { mutableStateOf(false) }
    var picking by remember { mutableStateOf<Picking?>(null) }
    val now by rememberNow()
    val leaving = OutingMoment(leaveDay, leaveMinute)
    val returning = OutingMoment(backDay, backMinute)
    val check = remember(form.rules, place, purpose, leaving, returning, now) { checkGeneralOuting(form.rules, place, purpose, leaving, returning) }
    val h24 = is24Hour()
    fun moment(m: OutingMoment, pattern: String = "EEE d MMM") = "${formatDay(m.day, pattern)}, ${clock(m.minute, h24)}"

    ApplySheet(
        "General outing",
        form.hostel,
        failure,
        sending,
        "outing.sheet.general",
        onReview = {
            if (listOf(check.place, check.purpose, check.leaving, check.returning).any { it != null }) {
                showErrors = true
                failure = "Check the fields marked in red."
            } else {
                failure = null
                confirming = true
            }
        },
        onDismiss = onDismiss,
    ) {
        TextInput(
            place, { place = it }, "Place of visit", form.rules.placeMaxLength.toInt(), check.place.takeIf { showErrors }, "outing.place",
            KeyboardOptions(KeyboardCapitalization.Words, imeAction = ImeAction.Next),
        )
        Spacer(Modifier.height(Space.sm))
        TextInput(
            purpose, { purpose = it }, "Purpose", form.rules.purposeMaxLength.toInt(), check.purpose.takeIf { showErrors }, "outing.purpose",
            KeyboardOptions(KeyboardCapitalization.Sentences, imeAction = ImeAction.Done),
        )
        Spacer(Modifier.height(Space.md))
        FieldLabel("Leaving", form.outWindow)
        MomentRow(leaving, check.leaving != null, h24, "outing.leaving", { picking = Picking.LeaveDate }, { picking = Picking.LeaveTime })
        // Timing problems show at once: they come from the pickers, not typing.
        FieldError(check.leaving, Modifier.testTag("outing.leaving.error"))
        Spacer(Modifier.height(Space.md))
        FieldLabel("Back by", form.inWindow)
        MomentRow(returning, check.returning != null, h24, "outing.returning", { picking = Picking.BackDate }, { picking = Picking.BackTime })
        FieldError(check.returning, Modifier.testTag("outing.returning.error"))
        check.away?.let {
            Text(it, Modifier.padding(top = Space.sm, start = 2.dp).testTag("outing.away"), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }

    when (val p = picking) {
        Picking.LeaveDate, Picking.BackDate -> {
            val leave = p == Picking.LeaveDate
            val range = if (leave) form.firstDay..form.lastDay else leaveDay..generalReturnLastDay(leaveDay)
            DayPicker(if (leave) leaveDay else backDay, range, onDismiss = { picking = null }) { day ->
                picking = null
                if (leave) {
                    // Keep the trip's length when the leaving day moves.
                    val gap = backDay - leaveDay
                    leaveDay = day
                    backDay = day + gap
                } else backDay = day
            }
        }
        Picking.LeaveTime, Picking.BackTime -> {
            val leave = p == Picking.LeaveTime
            TimeDialog(
                if (leave) "Leaving at" else "Back by",
                if (leave) leaveMinute else backMinute,
                if (leave) form.outWindow else form.inWindow,
                h24,
                onDismiss = { picking = null },
            ) { minute ->
                picking = null
                if (leave) leaveMinute = minute else backMinute = minute
            }
        }
        null -> {}
    }

    if (confirming) {
        ConfirmDialog(
            "Apply for general outing?",
            listOfNotNull(
                "Place" to tidy(place),
                "Purpose" to tidy(purpose),
                "Leaving" to moment(leaving),
                "Back by" to moment(returning),
                form.hostel?.let { "Room" to it },
            ),
            onApply = {
                confirming = false
                sending = true
                failure = null
                val application = GeneralOutingForm(tidy(place), tidy(purpose), leaving, returning)
                sendApplication(
                    "general",
                    { id, core -> core.applyGeneralOuting(id, application) },
                    onResult = { result -> if (result.done) { haptics.confirm(); onSent(result) } else { haptics.reject(); failure = result.message } },
                    onError = { message ->
                        failure = message
                        onMaybeSent()
                    },
                    onDone = { sending = false },
                )
            },
            onEdit = { confirming = false },
        )
    }
}

@Composable
private fun MomentRow(moment: OutingMoment, error: Boolean, h24: Boolean, tag: String, onDate: () -> Unit, onTime: () -> Unit) {
    Row(Modifier.fillMaxWidth()) {
        PickerField(Lucide.Calendar, formatDay(moment.day, "EEE d MMM"), error, onDate, Modifier.weight(3f).testTag("$tag.date"))
        Spacer(Modifier.width(Space.sm))
        PickerField(Lucide.Clock, clock(moment.minute, h24), error, onTime, Modifier.weight(2f).testTag("$tag.time"))
    }
}

/** A day in [range] (epoch days) from the M3 date picker. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun DayPicker(day: Int, range: IntRange, onDismiss: () -> Unit, onPick: (Int) -> Unit) {
    val state = rememberDatePickerState(
        initialSelectedDateMillis = day.coerceIn(range) * DAY_MS,
        yearRange = formatDay(range.first, "yyyy").toInt()..formatDay(range.last, "yyyy").toInt(),
        selectableDates = object : SelectableDates {
            override fun isSelectableDate(utcTimeMillis: Long) = (utcTimeMillis / DAY_MS).toInt() in range
        },
    )
    DatePickerDialog(
        onDismissRequest = onDismiss,
        confirmButton = {
            TextButton({ state.selectedDateMillis?.let { onPick((it / DAY_MS).toInt()) } ?: onDismiss() }, Modifier.testTag("outing.picker.ok")) { Text("Set") }
        },
        modifier = Modifier.testTag("outing.picker.date"),
        dismissButton = { TextButton(onDismiss) { Text("Cancel") } },
    ) { DatePicker(state, showModeToggle = false) }
}

/** A time of day (minutes) from the M3 time picker; [hint] is the offered window. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TimeDialog(title: String, minute: Int, hint: String?, h24: Boolean, onDismiss: () -> Unit, onPick: (Int) -> Unit) {
    val state = rememberTimePickerState(minute / 60, minute % 60, h24)
    TimePickerDialog(
        onDismissRequest = onDismiss,
        confirmButton = { TextButton({ onPick(state.hour * 60 + state.minute) }, Modifier.testTag("outing.picker.ok")) { Text("Set") } },
        title = {
            androidx.compose.foundation.layout.Column {
                Text(title)
                hint?.let { Text(it, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
        },
        modifier = Modifier.testTag("outing.picker.time"),
        dismissButton = { TextButton(onDismiss) { Text("Cancel") } },
    ) { TimePicker(state) }
}

/** Collapses runs of whitespace, as VTOP's form does (and Rust does again). */
internal fun tidy(text: String) = text.trim().split(Regex("\\s+")).filter { it.isNotEmpty() }.joinToString(" ")
