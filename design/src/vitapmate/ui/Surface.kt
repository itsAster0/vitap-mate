package vitapmate.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.dp

/**
 * The standard raised surface: card colour (white on the grey page in light), large radius, flat
 * like an M3 card. [borderColor] draws a hairline, e.g. to mark what is live; it is painted
 * behind [content]. Pass [onPress] to make it tappable with a gentle press-scale.
 */
@Composable
fun Surface(
    modifier: Modifier = Modifier,
    onPress: (() -> Unit)? = null,
    padding: PaddingValues = PaddingValues(Space.lg),
    color: Color? = null,
    borderColor: Color? = null,
    semanticsLabel: String? = null,
    content: @Composable () -> Unit,
) {
    val fill = color ?: AppTheme.colors.card
    val box = @Composable { m: Modifier ->
        Box(
            m.drawBehind {
                val r = Radii.lg.toPx()
                drawRoundRect(fill, cornerRadius = CornerRadius(r))
                borderColor?.let {
                    val w = 1.dp.toPx()
                    drawRoundRect(it, Offset(w / 2, w / 2), Size(size.width - w, size.height - w), CornerRadius(r - w / 2), Stroke(w))
                }
            }.padding(padding),
        ) { content() }
    }
    if (onPress == null) box(modifier) else PressScale(onPress, modifier, semanticsLabel) { box(Modifier) }
}
