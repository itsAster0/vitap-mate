package vitapmate.features.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import vitapmate.ui.Space
import vitapmate.ui.Tone

/** A tone's circle with an icon or initials, a title and lines under it (hostel, parent, mentor). */
@Composable
internal fun Headline(tone: Tone, title: String, lines: List<String>, icon: ImageVector? = null, initials: String = "", size: Dp = 44.dp) {
    Row(horizontalArrangement = Arrangement.spacedBy(Space.md), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(size).background(tone.subtle, CircleShape), Alignment.Center) {
            if (icon != null) Icon(icon, null, Modifier.size(20.dp), tint = tone.onSubtle)
            else Text(initials, style = MaterialTheme.typography.titleSmall, color = tone.onSubtle)
        }
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleMedium, color = MaterialTheme.colorScheme.onSurface)
            lines.forEach { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
        }
    }
}
