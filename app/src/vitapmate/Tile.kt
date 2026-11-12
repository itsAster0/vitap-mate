package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme

/**
 * A big square segment of a [ListGroup], two to a row (Connected apps' Add, Server's session and
 * password): [icon] in [shape] at the top, [title] and [text] at the bottom, then [action] (a
 * button); [primary] fills it.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun Tile(
    title: String,
    tag: String,
    icon: ImageVector,
    shape: Shape,
    text: String,
    primary: Boolean,
    modifier: Modifier,
    onClick: (() -> Unit)? = null,
    action: @Composable ColumnScope.() -> Unit = {},
) {
    val scheme = MaterialTheme.colorScheme
    val content = if (primary) scheme.onPrimary else scheme.onSurface
    Column(
        modifier.fillMaxHeight().heightIn(min = 150.dp).clip(ListItemDefaults.segmentedShapes(1, 3).shape).background(if (primary) scheme.primary else AppTheme.colors.card)
            .then(if (onClick != null) Modifier.clickable(role = Role.Button, onClick = onClick) else Modifier).testTag(tag)
            .padding(start = 16.dp, top = 16.dp, end = 16.dp, bottom = 18.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Box(Modifier.size(50.dp).background(if (primary) scheme.onPrimary else scheme.secondaryContainer, shape), Alignment.Center) {
            Icon(icon, null, Modifier.size(24.dp), tint = if (primary) scheme.primary else scheme.onSecondaryContainer)
        }
        Spacer(Modifier.weight(1f).heightIn(min = 14.dp))
        Text(title, style = MaterialTheme.typography.titleMediumEmphasized, color = content)
        Text(text, style = MaterialTheme.typography.bodySmall, color = if (primary) content.copy(alpha = 0.72f) else scheme.onSurfaceVariant)
        action()
    }
}
