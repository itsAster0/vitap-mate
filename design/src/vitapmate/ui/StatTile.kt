package vitapmate.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp

/** A number with its label (e.g. "82%" / "Overall"), on the [tone]'s fill or a neutral one. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun StatTile(
    value: String,
    label: String,
    modifier: Modifier = Modifier,
    tone: Tone? = null,
    icon: ImageVector? = null,
) {
    val scheme = MaterialTheme.colorScheme
    val fg = tone?.onSubtle ?: scheme.onSurface
    Column(modifier.background(tone?.subtle ?: scheme.surfaceContainerHigh, MaterialTheme.shapes.medium).padding(Space.md)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            icon?.let {
                Icon(it, null, Modifier.size(14.dp), tint = tone?.base ?: scheme.onSurfaceVariant)
                Spacer(Modifier.width(Space.xs))
            }
            Text(label, style = MaterialTheme.typography.labelMedium, color = fg.copy(alpha = 0.75f), maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
        Text(value, style = MaterialTheme.typography.titleLargeEmphasized.nums(), color = fg, maxLines = 1)
    }
}
