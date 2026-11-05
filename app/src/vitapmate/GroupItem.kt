package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SegmentedListItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone

/**
 * One row: an icon in a tonal circle, a title, an optional one-line [supporting] text, a trailing
 * slot (switch, chevron, value). In a [ListGroup] it is a segment on the card colour; alone it
 * has no container. [tone] colours the icon and title (e.g. danger for Sign out).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun GroupItem(
    title: String,
    tag: String,
    icon: ImageVector? = null,
    supporting: String? = null,
    enabled: Boolean = true,
    onClick: (() -> Unit)? = null,
    trailing: @Composable (() -> Unit)? = null,
    tone: Tone? = null,
) {
    val scheme = MaterialTheme.colorScheme
    val inGroup = LocalInGroup.current
    val container = if (inGroup) AppTheme.colors.card else Color.Transparent
    val headline = @Composable {
        Text(title, style = MaterialTheme.typography.bodyLarge, color = tone?.onSubtle ?: scheme.onSurface, maxLines = 2, overflow = TextOverflow.Ellipsis)
    }
    val support: (@Composable () -> Unit)? = supporting?.let {
        { Text(it, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis) }
    }
    val leading: (@Composable () -> Unit)? = icon?.let {
        {
            Box(Modifier.size(40.dp).background(tone?.subtle ?: scheme.secondaryContainer, CircleShape), Alignment.Center) {
                Icon(it, null, Modifier.size(20.dp), tint = tone?.base ?: scheme.onSecondaryContainer)
            }
        }
    }
    // Middle-segment corners everywhere: the group's clip rounds its first and last rows.
    val shapes = ListItemDefaults.segmentedShapes(1, 3)
    if (onClick != null) {
        SegmentedListItem(
            onClick = onClick,
            shapes = shapes,
            modifier = Modifier.testTag(tag),
            enabled = enabled,
            leadingContent = leading,
            trailingContent = trailing,
            supportingContent = support,
            colors = ListItemDefaults.segmentedColors(containerColor = container),
            contentPadding = if (inGroup) ListItemDefaults.ContentPadding else PaddingValues(vertical = 8.dp),
            content = headline,
        )
    } else {
        // Laid out like the segmented item (padding, 12 dp gaps), with no press state.
        Row(
            Modifier.fillMaxWidth().heightIn(min = if (supporting == null) 56.dp else 72.dp).clip(shapes.shape).background(container).testTag(tag)
                .padding(if (inGroup) ListItemDefaults.ContentPadding else PaddingValues(vertical = 8.dp)),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Row(Modifier.weight(1f).alpha(if (enabled) 1f else 0.5f), horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                leading?.invoke()
                Column(Modifier.weight(1f)) {
                    headline()
                    support?.invoke()
                }
            }
            trailing?.invoke()
        }
    }
}
