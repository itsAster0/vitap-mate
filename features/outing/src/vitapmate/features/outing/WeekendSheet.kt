package vitapmate.features.outing

import vitapmate.ui.rememberHaptics
import android.content.Context
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import vitapmate.core.OutingResult
import vitapmate.core.WeekendOuting
import vitapmate.core.WeekendOutingForm
import vitapmate.core.checkWeekendOuting
import vitapmate.ui.Space
import vitapmate.ui.formatDay
import vitapmate.ui.rememberNow

/** The last contact number used, so it needs no typing every week (Flutter's key). */
private const val CONTACT = "outing_contact_number"

private fun prefs(context: Context) = context.getSharedPreferences("settings", Context.MODE_PRIVATE)

/**
 * The weekend outing form: a day VTOP offers (from tomorrow), a time slot, a place, the purpose
 * and a contact number (the last one used filled in). Checks from Rust ([checkWeekendOuting]),
 * shown after the first Review; Review asks to confirm, then sends once (flow `outing.apply`).
 */
@OptIn(ExperimentalLayoutApi::class)
@Composable
internal fun WeekendSheet(form: WeekendOuting, onDismiss: () -> Unit, onMaybeSent: () -> Unit, onSent: (OutingResult) -> Unit) {
    val context = LocalContext.current
    val haptics = rememberHaptics()
    var day by rememberSaveable { mutableStateOf(form.dates.firstOrNull()) }
    var slot by rememberSaveable { mutableStateOf(form.slots.firstOrNull()?.value) }
    var place by rememberSaveable { mutableStateOf(form.places.firstOrNull()?.value) }
    var purpose by rememberSaveable { mutableStateOf("") }
    var contact by rememberSaveable { mutableStateOf(prefs(context).getString(CONTACT, null).orEmpty()) }
    var showErrors by rememberSaveable { mutableStateOf(false) }
    var failure by rememberSaveable { mutableStateOf<String?>(null) }
    var sending by remember { mutableStateOf(false) }
    var confirming by remember { mutableStateOf(false) }
    val now by rememberNow()
    val check = remember(form.rules, purpose, day, contact, now) { checkWeekendOuting(form.rules, purpose, day, contact) }
    fun shown(error: String?) = error.takeIf { showErrors }

    ApplySheet(
        "Weekend outing",
        form.hostel,
        failure,
        sending,
        "outing.sheet.weekend",
        onReview = {
            if (listOf(check.purpose, check.day, check.contact).any { it != null } || slot == null || place == null) {
                showErrors = true
                failure = "Check the fields marked in red."
            } else {
                failure = null
                confirming = true
            }
        },
        onDismiss = onDismiss,
    ) {
        FieldLabel("Day", form.daysLabel)
        FlowRow(horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
            form.dates.forEach { d ->
                ChoicePill(formatDay(d, "d MMM"), d == day, { day = d }, Modifier.testTag("outing.day.$d"), caption = formatDay(d, "EEE").uppercase())
            }
        }
        FieldError(shown(check.day))
        Spacer(Modifier.height(Space.md))
        FieldLabel("Time slot")
        // Two a row, as tall as the taller one, so a slot can wrap (after the dash, never inside a time).
        Column(verticalArrangement = Arrangement.spacedBy(Space.sm)) {
            form.slots.withIndex().chunked(2).forEach { row ->
                Row(Modifier.fillMaxWidth().height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                    row.forEach { (i, s) ->
                        ChoicePill(s.label.replace(" AM", "\u00A0AM").replace(" PM", "\u00A0PM"), s.value == slot, { slot = s.value }, Modifier.weight(1f).fillMaxHeight().testTag("outing.slot.$i"), lines = 2)
                    }
                    if (row.size == 1) Spacer(Modifier.weight(1f))
                }
            }
        }
        Spacer(Modifier.height(Space.md))
        FieldLabel("Place")
        FlowRow(horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
            form.places.forEachIndexed { i, p ->
                ChoicePill(p.label, p.value == place, { place = p.value }, Modifier.testTag("outing.place.$i"))
            }
        }
        Spacer(Modifier.height(Space.md))
        TextInput(
            purpose, { purpose = it }, "Purpose", form.rules.purposeMaxLength.toInt(), shown(check.purpose), "outing.purpose",
            KeyboardOptions(KeyboardCapitalization.Sentences, imeAction = ImeAction.Next),
        )
        Spacer(Modifier.height(Space.sm))
        TextInput(
            contact, { value -> if (value.all(Char::isDigit)) contact = value }, "Your contact number", 10, shown(check.contact), "outing.contact",
            KeyboardOptions(keyboardType = KeyboardType.Phone, imeAction = ImeAction.Done),
            counter = false,
        )
        form.parentContact?.let {
            Text("Parent contact on file: $it", Modifier.padding(start = 2.dp), style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }

    if (confirming) {
        val d = day
        val s = slot
        val p = place
        if (d == null || s == null || p == null) {
            confirming = false
            return
        }
        ConfirmDialog(
            "Apply for weekend outing?",
            listOf(
                "Day" to formatDay(d, "EEEE d MMM"),
                "Slot" to (form.slots.firstOrNull { it.value == s }?.label ?: s),
                "Place" to (form.places.firstOrNull { it.value == p }?.label ?: p),
                "Purpose" to tidy(purpose),
                "Contact" to contact.trim(),
            ),
            onApply = {
                confirming = false
                sending = true
                failure = null
                val application = WeekendOutingForm(p, tidy(purpose), d, s, contact.trim())
                sendApplication(
                    "weekend",
                    { id, core -> core.applyWeekendOuting(id, application) },
                    onResult = { result ->
                        if (result.done) {
                            haptics.confirm()
                            prefs(context).edit().putString(CONTACT, application.contact).apply()
                            onSent(result)
                        } else failure = result.message
                    },
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
