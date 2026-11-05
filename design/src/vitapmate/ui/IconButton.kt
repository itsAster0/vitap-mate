package vitapmate.ui

import androidx.compose.foundation.layout.size
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.OutlinedIconButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp

/** An outlined M3 icon button (44 dp, 18 dp icon) whose shape morphs on press. */
@Composable
fun IconButton(icon: ImageVector, onPress: (() -> Unit)?, label: String, modifier: Modifier = Modifier) =
    IconButton(onPress, label, modifier) { Icon(icon, label, Modifier.size(18.dp)) }

/** [IconButton] with its own 18 dp content, e.g. an animated icon. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun IconButton(onPress: (() -> Unit)?, label: String, modifier: Modifier = Modifier, content: @Composable () -> Unit) =
    OutlinedIconButton(
        onPress ?: {},
        modifier = modifier.size(44.dp).semantics { contentDescription = label },
        enabled = onPress != null,
        shapes = IconButtonDefaults.shapes(),
        border = IconButtonDefaults.outlinedIconButtonBorder(onPress != null).copy(width = 1.dp),
        content = content,
    )
