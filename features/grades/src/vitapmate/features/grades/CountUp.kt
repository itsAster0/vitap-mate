package vitapmate.features.grades

import androidx.compose.animation.core.Animatable
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle

/** [value] with two decimals, counting up from 0 when first shown and easing to each new value. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun CountUp(value: Double, style: TextStyle, modifier: Modifier = Modifier, color: Color = Color.Unspecified) {
    val shown = remember { Animatable(0f) }
    val spec = MaterialTheme.motionScheme.slowEffectsSpec<Float>()
    LaunchedEffect(value) { shown.animateTo(value.toFloat(), spec) }
    // The last frame lands exactly on the value, not the float's rounding of it.
    val text = if (shown.isRunning) shown.value.toDouble().fixed2() else value.fixed2()
    Text(text, modifier, style = style, color = color, maxLines = 1)
}
