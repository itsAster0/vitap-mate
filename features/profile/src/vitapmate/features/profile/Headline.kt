package vitapmate.features.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import vitapmate.ui.Tone

/** A tone's shape with an icon or initials, a title and lines under it (hostel, parent). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun Headline(tone: Tone, title: String, lines: List<String>, icon: ImageVector? = null, initials: String = "", size: Dp = 42.dp, shape: Shape = CircleShape) {
    Row(horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.CenterVertically) {
        if (icon != null) IconCircle(icon, tone, size, shape)
        else Box(Modifier.size(size).background(tone.subtle, shape), Alignment.Center) {
            Text(initials, style = MaterialTheme.typography.titleSmallEmphasized, color = tone.onSubtle)
        }
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleMediumEmphasized, color = MaterialTheme.colorScheme.onSurface)
            lines.forEach { Text(it, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant) }
        }
    }
}
