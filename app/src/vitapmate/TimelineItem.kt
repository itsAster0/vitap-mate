package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme

/**
 * One event on a timeline (Server's sign-ins, a key's last uses): a marker on a vertical line, a
 * [title], an optional [supporting] reason and the time [at] on the right. A [failed] event has a
 * square warning marker and title; others a round success one. A segment in a [ListGroup].
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun TimelineItem(title: String, tag: String, at: String, supporting: String? = null, failed: Boolean = false) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val line = scheme.outlineVariant
    Row(
        Modifier.fillMaxWidth().height(IntrinsicSize.Min).heightIn(min = 56.dp).clip(ListItemDefaults.segmentedShapes(1, 3).shape)
            .background(if (LocalInGroup.current) AppTheme.colors.card else Color.Transparent).testTag(tag).padding(horizontal = 16.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(
            Modifier.width(40.dp).fillMaxHeight().drawBehind { drawLine(line, Offset(size.width / 2, 0f), Offset(size.width / 2, size.height), 2.dp.toPx()) },
            Alignment.Center,
        ) {
            Box(Modifier.size(14.dp).background(if (failed) tones.warning.base else tones.success.base, if (failed) RoundedCornerShape(4.dp) else CircleShape))
        }
        Column(Modifier.weight(1f).padding(vertical = 8.dp)) {
            Text(title, style = MaterialTheme.typography.bodyLarge, color = if (failed) tones.warning.onSubtle else scheme.onSurface)
            supporting?.let { Text(it, style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant) }
        }
        TrailingValue(at)
    }
}
