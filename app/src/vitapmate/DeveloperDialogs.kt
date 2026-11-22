package vitapmate

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.input.TextFieldLineLimits
import androidx.compose.foundation.text.input.rememberTextFieldState
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import vitapmate.core.Core
import vitapmate.ui.AppDialog
import vitapmate.ui.PasswordField

/** Username and password for [Core.devSetAccount]; the password is never saved in instance state. */
@Composable
internal fun CredentialsDialog(onDismiss: () -> Unit, onSave: (String, String) -> Unit) {
    val username = rememberTextFieldState()
    var password by remember { mutableStateOf("") }
    val ok = username.text.isNotBlank() && password.isNotEmpty()
    AppDialog(
        onDismiss,
        "Set credentials",
        "Save",
        {
            onSave(username.text.toString().trim(), password)
            onDismiss()
        },
        confirmEnabled = ok,
        confirmTag = "dev.account.save",
    ) {
        OutlinedTextField(username, Modifier.testTag("dev.account.username"), label = { Text("Username") }, lineLimits = TextFieldLineLimits.SingleLine)
        PasswordField(password, { password = it }, "Password", "dev.account.password", Modifier.padding(top = 12.dp))
    }
}

/** An M3 dialog editing one value: Save (when [check] passes), Default, Cancel. */
@Composable
internal fun TextDialog(
    title: String,
    note: String,
    current: String,
    number: Boolean,
    check: (String) -> Boolean,
    onDismiss: () -> Unit,
    onDefault: () -> Unit,
    onSave: (String) -> Unit,
) {
    val text = rememberTextFieldState(current)
    val value = text.text.toString()
    AppDialog(
        onDismiss,
        title,
        "Save",
        {
            onSave(value.trim())
            onDismiss()
        },
        confirmEnabled = check(value),
        confirmTag = "dev.edit.save",
        extra = {
            TextButton({
                onDefault()
                onDismiss()
            }, Modifier.testTag("dev.edit.default")) { Text("Default") }
        },
    ) {
        Text(note)
        OutlinedTextField(
            text,
            Modifier.padding(top = 12.dp).testTag("dev.edit.field"),
            isError = !check(value),
            keyboardOptions = KeyboardOptions(keyboardType = if (number) KeyboardType.Number else KeyboardType.Uri),
            lineLimits = TextFieldLineLimits.SingleLine,
        )
    }
}

/** The update screens to preview without Play; each closes the dialog. */
@Composable
internal fun UpdatePreviewDialog(onDismiss: () -> Unit) = AlertDialog(
    onDismissRequest = onDismiss,
    title = { Text("Preview an update") },
    text = {
        Column {
            Text("Levels 3 and 4 are Google Play's own dialogs and show only for a real update.")
            listOf(
                "Level 2: Update runs a fake download" to { PlayUpdate.preview(UpdatePill.Available) },
                "Update ready (end of levels 2 and 3)" to { PlayUpdate.preview(UpdatePill.Ready) },
                "Level 5: update required" to { PlayUpdate.preview(null, blocked = true) },
            ).forEachIndexed { i, (label, run) ->
                TextButton({ onDismiss(); run() }, Modifier.fillMaxWidth().padding(top = if (i == 0) 12.dp else 0.dp).testTag("dev.update_preview.$i")) { Text(label) }
            }
        }
    },
    confirmButton = { TextButton(onDismiss) { Text("Close") } },
)
