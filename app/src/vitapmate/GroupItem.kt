package vitapmate

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
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
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SegmentedListItem
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Lucide
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone

/**
 * One row: an icon in a tonal circle, a title, an optional one-line [supporting] text, a trailing
 * slot (switch, chevron, value). In a [ListGroup] it is a segment on the card colour; alone it
 * has no container. [tone] colours the icon and title (e.g. warning). [on] is the row's switch
 * ([ItemSwitch]) state: when on, the icon's circle morphs into a primary squircle (a [tone]
 * keeps its colours, so a warning still reads as one). [below] sits
 * under the supporting text. [lead] replaces the icon (e.g. a value badge).
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
    on: Boolean = false,
    below: (@Composable () -> Unit)? = null,
    highlight: Boolean = false,
    iconShape: Shape? = null,
    lead: (@Composable () -> Unit)? = null,
) {
    val scheme = MaterialTheme.colorScheme
    val motion = MaterialTheme.motionScheme
    val inGroup = LocalInGroup.current
    val tones = AppTheme.tones
    val container = if (highlight) tones.accentTone.subtle else if (inGroup) AppTheme.colors.card else Color.Transparent
    val headline = @Composable {
        Text(title, style = MaterialTheme.typography.bodyLarge, color = tone?.onSubtle ?: scheme.onSurface, maxLines = 2, overflow = TextOverflow.Ellipsis)
    }
    val support: (@Composable () -> Unit)? = if (supporting == null && below == null) null else {
        {
            Column {
                if (supporting != null) Text(supporting, style = MaterialTheme.typography.bodyMedium, color = if (highlight) tones.accentTone.onSubtle else scheme.onSurfaceVariant, maxLines = 2, overflow = TextOverflow.Ellipsis)
                below?.invoke()
            }
        }
    }
    val leading: (@Composable () -> Unit)? = lead ?: icon?.let {
        {
            val lit = on && tone == null
            val corner by animateDpAsState(if (lit) 14.dp else 20.dp, motion.defaultSpatialSpec())
            val fill by animateColorAsState(if (lit) scheme.primary else if (highlight) tones.accent else tone?.subtle ?: scheme.secondaryContainer, motion.defaultEffectsSpec())
            val tint by animateColorAsState(if (lit) scheme.onPrimary else if (highlight) tones.onAccent else tone?.base ?: scheme.onSecondaryContainer, motion.defaultEffectsSpec())
            Box(Modifier.size(40.dp).background(fill, if (iconShape != null && !lit) iconShape else RoundedCornerShape(corner)), Alignment.Center) {
                Icon(it, null, Modifier.size(20.dp), tint = tint)
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

/** A [GroupItem]'s switch, with a check in its thumb when on; pass the same state as [GroupItem]'s `on`. */
@Composable
fun ItemSwitch(checked: Boolean, onCheckedChange: ((Boolean) -> Unit)?, enabled: Boolean = true) = Switch(
    checked,
    onCheckedChange,
    enabled = enabled,
    thumbContent = if (checked) ({ Icon(Lucide.Check, null, Modifier.size(SwitchDefaults.IconSize)) }) else null,
)
