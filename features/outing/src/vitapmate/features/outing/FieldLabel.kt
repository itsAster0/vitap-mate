package vitapmate.features.outing

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import vitapmate.ui.Space
import vitapmate.ui.nums

/** A caption over a group of choices, with an optional note at its end. */
@Composable
internal fun FieldLabel(text: String, trailing: String? = null) {
    Row(Modifier.fillMaxWidth().padding(start = 2.dp, bottom = Space.sm - 2.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(text, Modifier.weight(1f), style = MaterialTheme.typography.titleSmall)
        trailing?.let { Text(it, style = MaterialTheme.typography.labelMedium.nums(), color = MaterialTheme.colorScheme.onSurfaceVariant) }
    }
}
