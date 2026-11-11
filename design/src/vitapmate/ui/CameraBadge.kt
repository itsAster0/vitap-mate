package vitapmate.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Camera
import com.composables.icons.lucide.Lucide

/** The small camera at an [Avatar]'s corner on an accent-fill header: the photo menu is a tap away. */
@Composable
fun CameraBadge(size: Dp, modifier: Modifier = Modifier) {
    val tones = AppTheme.tones
    Box(modifier.size(size).background(tones.accent, CircleShape).border(3.dp, tones.accentTone.subtle, CircleShape), Alignment.Center) {
        Icon(Lucide.Camera, null, Modifier.size(size * 0.47f), tint = tones.onAccent)
    }
}
