package vitapmate.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** A small filled circle, or a hollow one with a [ring]. */
@Composable
fun Dot(modifier: Modifier = Modifier, size: Dp = 5.dp, color: Color = Color.Transparent, ring: Color? = null) {
    Box(
        modifier.size(size).background(color, CircleShape)
            .then(if (ring != null) Modifier.border(1.2.dp, ring, CircleShape) else Modifier),
    )
}
