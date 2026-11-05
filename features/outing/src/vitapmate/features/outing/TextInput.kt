package vitapmate.features.outing

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import vitapmate.ui.nums

/**
 * A one-line field with its [label], at most [max] characters (counted under it, or [error]
 * there in the error colour).
 */
@Composable
internal fun TextInput(
    value: String,
    onChange: (String) -> Unit,
    label: String,
    max: Int,
    error: String?,
    tag: String,
    keyboard: KeyboardOptions,
    counter: Boolean = true,
) {
    OutlinedTextField(
        value,
        { if (it.length <= max) onChange(it) },
        Modifier.fillMaxWidth().testTag(tag),
        label = { Text(label) },
        isError = error != null,
        singleLine = true,
        keyboardOptions = keyboard,
        shape = MaterialTheme.shapes.medium,
        supportingText = {
            Row {
                Text(error.orEmpty(), Modifier.weight(1f))
                if (counter) Text("${value.length}/$max", style = MaterialTheme.typography.labelSmall.nums())
            }
        },
    )
}
