package vitapmate.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** Thin horizontal bar that animates to [value] (0–1). */
@Composable
fun ProgressBar(value: Float, color: Color, modifier: Modifier = Modifier, height: Dp = 6.dp) {
    val v = remember { Animatable(0f) }
    LaunchedEffect(value) { v.animateTo(value.coerceIn(0f, 1f), tween(Motion.SLOW * 2, easing = EaseOutCubic)) }
    Box(modifier.fillMaxWidth().height(height).clip(RoundedCornerShape(height)).background(MaterialTheme.colorScheme.surfaceContainerHighest)) {
        Box(Modifier.fillMaxHeight().fillMaxWidth(v.value).background(color))
    }
}
