package vitapmate.ui

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.dp

/**
 * A card for one section of a screen: optional [title], [subtitle] and trailing [action] over
 * [content]. Card colour on the page (white in light), large [shape] by default, no border;
 * tappable with [onClick].
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SectionCard(
    modifier: Modifier = Modifier,
    title: String? = null,
    subtitle: String? = null,
    action: (@Composable () -> Unit)? = null,
    onClick: (() -> Unit)? = null,
    shape: Shape = MaterialTheme.shapes.large,
    content: @Composable ColumnScope.() -> Unit,
) {
    val colors = CardDefaults.cardColors(containerColor = AppTheme.colors.card)
    val body = @Composable {
        Column(Modifier.padding(Space.lg)) {
            if (title != null || action != null) {
                Row(Modifier.padding(bottom = if (subtitle == null) Space.md else 2.dp), verticalAlignment = Alignment.CenterVertically) {
                    Text(title.orEmpty(), Modifier.weight(1f), style = MaterialTheme.typography.titleMediumEmphasized)
                    action?.invoke()
                }
            }
            subtitle?.let {
                Text(it, Modifier.padding(bottom = Space.md), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            content()
        }
    }
    if (onClick != null) Card(onClick, modifier, shape = shape, colors = colors) { body() }
    else Card(modifier, shape, colors) { body() }
}
