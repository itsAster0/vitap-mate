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
import vitapmate.ui.AppTheme

/**
 * A rounded track with [percent] (0–100) filled in [color], a small gap between the fill and the
 * rest of the [track], and a tick at 75% standing out above and below. The fill springs out from
 * zero when first shown, and to a new value after. With [from], a dot marks where it stands now
 * (the calculator's what-if). [halo] is the colour behind it, ringing the tick and the dot.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun AttendanceLine(
    percent: Double,
    color: Color,
    modifier: Modifier = Modifier,
    thickness: Dp = 8.dp,
    from: Double? = null,
    track: Color = MaterialTheme.colorScheme.surfaceContainerHighest,
    halo: Color = AppTheme.colors.card,
) {
    val scheme = MaterialTheme.colorScheme
    var shown by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { shown = true }
    val fill by animateFloatAsState(
        if (shown) (percent / 100).toFloat().coerceIn(0f, 1f) else 0f,
        MaterialTheme.motionScheme.slowSpatialSpec(),
    )
    val ink = scheme.onSurface
    Canvas(modifier.fillMaxWidth().height(thickness + 8.dp)) {
        val h = thickness.toPx()
        val top = (size.height - h) / 2
        val radius = CornerRadius(h / 2)
        val gap = 3.dp.toPx()
        // The spring overshoots a little; never past the ends.
        val width = (size.width * fill).coerceIn(0f, size.width)
        val rest = if (width > 0f) width.coerceAtLeast(h) + gap else 0f
        if (rest < size.width) drawRoundRect(track, Offset(rest, top), Size(size.width - rest, h), radius)
        if (width > 0f) drawRoundRect(color, Offset(0f, top), Size(width.coerceAtLeast(h), h), radius)
        val x = size.width * 0.75f
        // Round caps reach half the width past the ends: the tick fills the height, its halo 2 dp more.
        val cap = 1.5.dp.toPx()
        drawLine(halo, Offset(x, cap), Offset(x, size.height - cap), 7.dp.toPx(), StrokeCap.Round)
        drawLine(ink, Offset(x, cap), Offset(x, size.height - cap), 3.dp.toPx(), StrokeCap.Round)
        from?.let {
            val r = 4.dp.toPx()
            val cx = (size.width * (it / 100).toFloat()).coerceIn(h / 2, size.width - h / 2)
            drawCircle(halo, r + 2.dp.toPx(), Offset(cx, size.height / 2))
            drawCircle(ink, r, Offset(cx, size.height / 2))
        }
    }
}
