package vitapmate.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.size
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.OutlinedButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.material3.Button as M3Button

enum class ButtonVariant { Primary, Outline }

/** Heights: [Md] 44, [Sm] 40. */
enum class ButtonSize(val min: Dp, val padding: PaddingValues, val gap: Dp) {
    Md(44.dp, PaddingValues(horizontal = 20.dp, vertical = 10.dp), 8.dp),
    Sm(40.dp, PaddingValues(horizontal = 16.dp, vertical = 8.dp), 6.dp),
}

/**
 * An M3 Expressive button (filled for [ButtonVariant.Primary], outlined otherwise) whose shape
 * morphs on press; optional 18 dp [prefix] icon; disabled when [onPress] is null.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun Button(
    onPress: (() -> Unit)?,
    modifier: Modifier = Modifier,
    variant: ButtonVariant = ButtonVariant.Primary,
    size: ButtonSize = ButtonSize.Md,
    prefix: ImageVector? = null,
    content: @Composable RowScope.() -> Unit,
) {
    val m = modifier.defaultMinSize(minHeight = size.min)
    val body: @Composable RowScope.() -> Unit = {
        prefix?.let { Icon(it, null, Modifier.size(18.dp)) }
        content()
    }
    val arrangement = Arrangement.spacedBy(size.gap, Alignment.CenterHorizontally)
    when (variant) {
        ButtonVariant.Primary -> M3Button(
            onPress ?: {}, modifier = m, enabled = onPress != null, shapes = ButtonDefaults.shapes(), contentPadding = size.padding,
        ) { Row(horizontalArrangement = arrangement, verticalAlignment = Alignment.CenterVertically, content = body) }
        ButtonVariant.Outline -> OutlinedButton(
            onPress ?: {}, modifier = m, enabled = onPress != null, shapes = ButtonDefaults.shapes(), contentPadding = size.padding,
        ) { Row(horizontalArrangement = arrangement, verticalAlignment = Alignment.CenterVertically, content = body) }
    }
}
