package vitapmate.features.outing

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MapPin
import vitapmate.ui.AppDialog
import vitapmate.ui.DialogRows

/** Shows what is about to be sent; Apply sends it, Edit goes back to the form. */
@Composable
internal fun ConfirmDialog(title: String, rows: List<Pair<String, String>>, onApply: () -> Unit, onEdit: () -> Unit) {
    AppDialog(
        onEdit,
        title,
        confirm = "Apply",
        onConfirm = onApply,
        modifier = Modifier.testTag("outing.confirm"),
        icon = Lucide.MapPin,
        confirmTag = "outing.confirm.apply",
        dismiss = "Edit",
        dismissTag = "outing.confirm.edit",
    ) { DialogRows(rows) }
}
