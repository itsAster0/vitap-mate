package vitapmate.ui

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** Placeholder block that pulses between two M3 surface levels while content loads; [width] null fills the width given. */
@Composable
fun Skeleton(modifier: Modifier = Modifier, width: Dp? = null, height: Dp = 14.dp, radius: Dp = Radii.sm) {
    val scheme = MaterialTheme.colorScheme
    val pulse by rememberInfiniteTransition("skeleton").animateFloat(
        0f, 1f, infiniteRepeatable(tween(1100), RepeatMode.Reverse), "pulse",
    )
    Box(
        modifier.then(if (width != null) Modifier.width(width) else Modifier).height(height).drawBehind {
            // Read in the draw phase, so the pulse doesn't recompose.
            val c = lerp(scheme.surfaceContainerHigh, scheme.surfaceContainerHighest, pulse)
            drawRoundRect(c, cornerRadius = CornerRadius(radius.toPx()))
        },
    )
}
