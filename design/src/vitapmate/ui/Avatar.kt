package vitapmate.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.Dp

/**
 * The student's avatar: the photo they chose ([AvatarPhoto]) clipped to [shape], else [letters]
 * on the primary container. Never VTOP's photo unless they picked it.
 */
@Composable
fun Avatar(letters: String, size: Dp, modifier: Modifier = Modifier, shape: Shape = CircleShape, style: TextStyle = MaterialTheme.typography.titleSmall) {
    val photo by rememberAvatarBitmap()
    val scheme = MaterialTheme.colorScheme
    Box(modifier.size(size).clip(shape).background(scheme.primaryContainer).testTag(if (photo != null) "avatar.photo" else "avatar.initial"), Alignment.Center) {
        val image = photo
        if (image != null) Image(image, null, Modifier.size(size), contentScale = ContentScale.Crop)
        else Text(letters.ifBlank { "•" }, style = style, color = scheme.onPrimaryContainer)
    }
}
