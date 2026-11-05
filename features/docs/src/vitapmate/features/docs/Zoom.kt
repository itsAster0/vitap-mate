package vitapmate.features.docs

import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animate
import androidx.compose.animation.core.spring
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateCentroid
import androidx.compose.foundation.gestures.calculatePan
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.unit.IntSize
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/** Pinch-zoom and pan of a viewer's content, 1×–5×; double tap toggles 2.5× with a spring. */
@Stable
internal class ZoomState(initial: Float, private val scope: CoroutineScope) {
    var scale by mutableFloatStateOf(initial.coerceIn(MIN, MAX))
        private set
    var offset by mutableStateOf(Offset.Zero)
        private set
    var size = IntSize.Zero
    private var running: Job? = null

    fun reset() = animateTo(1f, Offset.Zero)

    /** [factor] around [centroid], then moved by [pan]. */
    fun transform(factor: Float, centroid: Offset, pan: Offset) {
        running?.cancel()
        val next = (scale * factor).coerceIn(MIN, MAX)
        offset = clamp(centroid - (centroid - offset) * (next / scale) + pan, next)
        scale = next
    }

    fun toggle(at: Offset) = if (scale > 1.05f) reset() else animateTo(2.5f, clamp(at - at * 2.5f, 2.5f))

    private fun animateTo(s: Float, o: Offset) {
        running?.cancel()
        val fromScale = scale
        val fromOffset = offset
        running = scope.launch {
            animate(0f, 1f, animationSpec = spring(Spring.DampingRatioLowBouncy, Spring.StiffnessMediumLow)) { t, _ ->
                scale = fromScale + (s - fromScale) * t
                offset = fromOffset + (o - fromOffset) * t
            }
        }
    }

    private fun clamp(o: Offset, s: Float) = Offset(o.x.coerceIn(size.width * (1 - s), 0f), o.y.coerceIn(size.height * (1 - s), 0f))

    companion object {
        const val MIN = 1f
        const val MAX = 5f
    }
}

@Composable
internal fun rememberZoom(initial: Float = 1f): ZoomState {
    val scope = rememberCoroutineScope()
    return remember { ZoomState(initial, scope) }
}

/**
 * The gestures for [zoom] on the box holding the content. Two fingers always zoom and pan; one
 * finger pans only when zoomed, and with [list] only sideways, so the list keeps scrolling.
 */
internal fun Modifier.zoomGestures(zoom: ZoomState, list: Boolean): Modifier = this
    .onSizeChanged { zoom.size = it }
    .pointerInput(zoom) { detectTapGestures(onDoubleTap = zoom::toggle) }
    .pointerInput(zoom, list) {
        awaitEachGesture {
            awaitFirstDown(requireUnconsumed = false)
            do {
                val event = awaitPointerEvent(PointerEventPass.Initial)
                val fingers = event.changes.count { it.pressed }
                if (fingers > 1) {
                    zoom.transform(event.calculateZoom(), event.calculateCentroid(), event.calculatePan())
                    event.changes.forEach { it.consume() }
                } else if (fingers == 1 && zoom.scale > 1.01f) {
                    val pan = event.calculatePan()
                    zoom.transform(1f, Offset.Zero, if (list) Offset(pan.x, 0f) else pan)
                    if (!list) event.changes.forEach { it.consume() }
                }
            } while (event.changes.any { it.pressed })
        }
    }

/** Draws the content at [zoom]'s scale and offset. */
internal fun Modifier.zoomed(zoom: ZoomState): Modifier = graphicsLayer {
    transformOrigin = TransformOrigin(0f, 0f)
    scaleX = zoom.scale
    scaleY = zoom.scale
    translationX = zoom.offset.x
    translationY = zoom.offset.y
}
