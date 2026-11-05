package vitapmate.features.outing

import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import vitapmate.core.OutingRequest

/** Asks before [request] is cancelled on VTOP. */
@Composable
internal fun CancelDialog(request: OutingRequest, onDismiss: () -> Unit, onConfirm: () -> Unit) {
    AlertDialog(
        onDismiss,
        confirmButton = {
            TextButton(
                onConfirm,
                Modifier.testTag("outing.cancel.confirm"),
                colors = ButtonDefaults.textButtonColors(contentColor = MaterialTheme.colorScheme.error),
            ) { Text("Cancel request") }
        },
        modifier = Modifier.testTag("outing.cancel.dialog"),
        dismissButton = { TextButton(onDismiss, Modifier.testTag("outing.cancel.keep")) { Text("Keep it") } },
        title = { Text("Cancel this request?") },
        text = { Text(listOf(request.place, request.purpose).filter { it.isNotBlank() }.joinToString(" · ")) },
    )
}
