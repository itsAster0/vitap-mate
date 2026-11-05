package vitapmate.features.attendance

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/**
 * A rounded track with [percent] (0–100) filled in [color] and a small tick at 75%. The fill
 * springs out from zero when first shown, and to a new value after.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun AttendanceLine(percent: Double, color: Color, modifier: Modifier = Modifier, thickness: Dp = 6.dp) {
    val scheme = MaterialTheme.colorScheme
    var shown by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { shown = true }
    val fill by animateFloatAsState(
        if (shown) (percent / 100).toFloat().coerceIn(0f, 1f) else 0f,
        MaterialTheme.motionScheme.slowSpatialSpec(),
    )
    val track = scheme.surfaceContainerHighest
    val tick = scheme.onSurfaceVariant
    Canvas(modifier.fillMaxWidth().height(thickness + 6.dp)) {
        val h = thickness.toPx()
        val top = (size.height - h) / 2
        val radius = CornerRadius(h / 2)
        drawRoundRect(track, Offset(0f, top), Size(size.width, h), radius)
        // The spring overshoots a little; never past the ends.
        val width = (size.width * fill).coerceIn(0f, size.width)
        if (width > 0f) drawRoundRect(color, Offset(0f, top), Size(width.coerceAtLeast(h), h), radius)
        val x = size.width * 0.75f
        drawLine(tick, Offset(x, 0f), Offset(x, size.height), 2.dp.toPx(), StrokeCap.Round)
    }
}
