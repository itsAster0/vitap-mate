package vitapmate.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics

/** Makes [content] tappable; it shrinks slightly while pressed (on top of the tap bounce). */
@Composable
fun PressScale(
    onPress: () -> Unit,
    modifier: Modifier = Modifier,
    semanticsLabel: String? = null,
    scale: Float = 0.98f,
    content: @Composable () -> Unit,
) {
    Box(
        modifier.tappable(onPress, haptic = true, pressedScale = scale)
            .then(if (semanticsLabel != null) Modifier.semantics { contentDescription = semanticsLabel } else Modifier),
    ) { content() }
}
