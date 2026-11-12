package vitapmate.features.outing

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import vitapmate.ui.Space
import vitapmate.ui.nums

/** Looks like a filled field in [shape] (outlined in red on an [error]); tapping it opens a picker. */
@Composable
internal fun PickerField(icon: ImageVector, text: String, error: Boolean, onPress: () -> Unit, modifier: Modifier, shape: Shape) {
    val scheme = MaterialTheme.colorScheme
    Surface(
        onClick = onPress,
        modifier = modifier.height(48.dp),
        shape = shape,
        color = scheme.surfaceContainerHigh,
        border = if (error) BorderStroke(2.dp, scheme.error) else null,
    ) {
        Row(Modifier.padding(horizontal = Space.md), verticalAlignment = Alignment.CenterVertically) {
            Icon(icon, null, Modifier.size(16.dp), tint = scheme.onSurfaceVariant)
            Spacer(Modifier.width(Space.sm))
            Text(text, style = MaterialTheme.typography.bodyMedium.nums(), maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}
