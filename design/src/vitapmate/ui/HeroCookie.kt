package vitapmate.ui

import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawOutline
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp

/**
 * A large faint cookie behind an accent-fill header, cut off by its top end corner (Profile's
 * hero, Settings' header). Drawn, not laid out, so it never sizes the header; clip the header.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun Modifier.heroCookie(): Modifier {
    val color = AppTheme.tones.accent.copy(alpha = 0.1f)
    val shape = MaterialShapes.Cookie9Sided.toShape()
    return drawWithCache {
        val side = 220.dp.toPx()
        val outline = shape.createOutline(Size(side, side), layoutDirection, this)
        val left = if (layoutDirection == LayoutDirection.Ltr) size.width - side + 70.dp.toPx() else -70.dp.toPx()
        onDrawBehind { translate(left, -80.dp.toPx()) { drawOutline(outline, color) } }
    }
}
