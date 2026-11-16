package vitapmate.features.outing

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.Ban
import com.composables.icons.lucide.Lucide
import vitapmate.core.OutingRequest
import vitapmate.ui.AppDialog
import vitapmate.ui.DialogRows

/** Asks before [request] is cancelled on VTOP. */
@Composable
internal fun CancelDialog(request: OutingRequest, onDismiss: () -> Unit, onConfirm: () -> Unit) {
    AppDialog(
        onDismiss,
        "Cancel this request?",
        confirm = "Cancel request",
        onConfirm = onConfirm,
        modifier = Modifier.testTag("outing.cancel.dialog"),
        icon = Lucide.Ban,
        danger = true,
        confirmTag = "outing.cancel.confirm",
        dismiss = "Keep it",
        dismissTag = "outing.cancel.keep",
    ) { DialogRows(listOf("" to listOf(request.place, request.purpose).filter { it.isNotBlank() }.joinToString(" · "))) }
}
