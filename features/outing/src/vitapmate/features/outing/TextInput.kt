package vitapmate.features.outing

import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.ui.nums

/**
 * A one-line field on a filled tile with its [label], at most [max] characters (counted under
 * it, or [error] there in the error colour, with a red outline).
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
    val scheme = MaterialTheme.colorScheme
    val shape = MaterialTheme.shapes.large
    val fill = scheme.surfaceContainerHigh
    Column(Modifier.fillMaxWidth()) {
        TextField(
            value,
            { if (it.length <= max) onChange(it) },
            Modifier.fillMaxWidth().then(if (error != null) Modifier.border(2.dp, scheme.error, shape) else Modifier).testTag(tag),
            label = { Text(label) },
            isError = error != null,
            singleLine = true,
            keyboardOptions = keyboard,
            shape = shape,
            colors = TextFieldDefaults.colors(
                focusedContainerColor = fill,
                unfocusedContainerColor = fill,
                errorContainerColor = fill,
                focusedIndicatorColor = Color.Transparent,
                unfocusedIndicatorColor = Color.Transparent,
                errorIndicatorColor = Color.Transparent,
            ),
        )
        // Under the tile, so the red outline holds only the field.
        Row(Modifier.padding(start = 16.dp, top = 4.dp, end = 16.dp)) {
            val color = if (error != null) scheme.error else scheme.onSurfaceVariant
            Text(error.orEmpty(), Modifier.weight(1f), style = MaterialTheme.typography.bodySmall, color = color)
            if (counter) Text("${value.length}/$max", style = MaterialTheme.typography.labelSmall.nums(), color = color)
        }
    }
}
