package vitapmate.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp

/** The muted tone [ToneBadge] uses when none is given. */
val neutralTone: Tone
    @Composable get() = AppTheme.colors.run { Tone(mutedForeground, secondary, mutedForeground) }

/** Small uppercase label on a tinted fill, e.g. NOW, LAB, 92%. */
@Composable
fun ToneBadge(
    label: String,
    modifier: Modifier = Modifier,
    tone: Tone = neutralTone,
    icon: ImageVector? = null,
    solid: Boolean = false,
) {
    val fg = if (solid) AppTheme.colors.background else tone.onSubtle
    Row(
        modifier
            .background(if (solid) tone.base else tone.subtle, RoundedCornerShape(Radii.pill))
            .padding(horizontal = 8.dp, vertical = 3.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        icon?.let {
            Icon(it, null, Modifier.size(11.dp), tint = fg)
            Spacer(Modifier.width(4.dp))
        }
        Text(label, style = numStyle(10.5.sp, FontWeight.W700, fg, 1.3.em, 0.4.sp))
    }
}
