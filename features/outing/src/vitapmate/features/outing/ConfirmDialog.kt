package vitapmate.features.outing

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.width
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.ui.Space

/** Shows what is about to be sent; Apply sends it, Edit goes back to the form. */
@Composable
internal fun ConfirmDialog(title: String, rows: List<Pair<String, String>>, onApply: () -> Unit, onEdit: () -> Unit) {
    AlertDialog(
        onEdit,
        confirmButton = { TextButton(onApply, Modifier.testTag("outing.confirm.apply")) { Text("Apply") } },
        modifier = Modifier.testTag("outing.confirm"),
        dismissButton = { TextButton(onEdit, Modifier.testTag("outing.confirm.edit")) { Text("Edit") } },
        title = { Text(title) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(Space.xs + 2.dp)) {
                rows.forEach { (label, value) ->
                    Row {
                        Text(label, Modifier.width(76.dp), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                        Text(value, style = MaterialTheme.typography.titleSmall)
                    }
                }
            }
        },
    )
}
