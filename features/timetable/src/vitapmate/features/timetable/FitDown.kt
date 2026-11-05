package vitapmate.features.timetable

import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.layout
import androidx.compose.ui.unit.Constraints
import kotlin.math.roundToInt

/**
 * Flutter's `FittedBox(fit: scaleDown)`: lays the content out at its natural width and shrinks it
 * to fit, never grows it. [end] anchors it right (`alignment: topRight`), else left.
 * TODO: move to ui/ if another screen needs it.
 */
fun Modifier.fitDown(end: Boolean = false): Modifier = layout { measurable, constraints ->
    val p = measurable.measure(constraints.copy(minWidth = 0, maxWidth = Constraints.Infinity))
    val scale = if (constraints.hasBoundedWidth && p.width > constraints.maxWidth) constraints.maxWidth / p.width.toFloat() else 1f
    val w = (p.width * scale).roundToInt()
    val h = (p.height * scale).roundToInt()
    layout(w, h) {
        p.placeWithLayer(0, 0) {
            scaleX = scale
            scaleY = scale
            transformOrigin = androidx.compose.ui.graphics.TransformOrigin(if (end) 1f else 0f, 0f)
            translationX = if (end) (w - p.width).toFloat() else 0f
        }
    }
}
