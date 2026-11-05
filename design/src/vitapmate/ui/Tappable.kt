package vitapmate.ui

import androidx.compose.animation.core.EaseOut
import androidx.compose.animation.core.EaseOutCubic
import androidx.compose.animation.core.EaseOutQuart
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.Role

/**
 * forui's `FTappable`: a ripple-free tap that bounces to 0.97 while pressed (100 ms easeOutQuart
 * down, 100 ms easeOutCubic up). [pressedScale] stacks a further scale (120 ms easeOut), like
 * PressScale's 0.98. [haptic] clicks on press. Pass [interaction] to read the pressed state.
 */
@Composable
fun Modifier.tappable(
    onPress: (() -> Unit)?,
    label: String? = null,
    role: Role? = null,
    haptic: Boolean = false,
    pressedScale: Float = 1f,
    interaction: MutableInteractionSource = remember { MutableInteractionSource() },
): Modifier {
    val view = LocalView.current
    val pressed by interaction.collectIsPressedAsState()
    val bounce by animateFloatAsState(
        if (pressed) 0.97f else 1f,
        tween(100, easing = if (pressed) EaseOutQuart else EaseOutCubic),
    )
    val extra by animateFloatAsState(if (pressed) pressedScale else 1f, tween(Motion.FAST, easing = EaseOut))
    return graphicsLayer { scaleX = bounce * extra; scaleY = bounce * extra }
        .clickable(interaction, null, onPress != null, label, role) {
            if (haptic) Haptics(view).tick()
            onPress?.invoke()
        }
}
