package vitapmate.features.exams

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.BookOpenCheck
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Space

/** "2 days to prepare" as a pill on a dashed line, between two exam days. */
@Composable
internal fun GapRow(days: Int, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val line = scheme.outlineVariant
    Box(
        modifier.fillMaxWidth().padding(top = Space.xs).drawBehind {
            val y = size.height / 2
            drawLine(line, Offset(0f, y), Offset(size.width, y), 1.dp.toPx(), pathEffect = PathEffect.dashPathEffect(floatArrayOf(4.dp.toPx(), 4.dp.toPx())))
        },
        Alignment.Center,
    ) {
        Row(
            Modifier.background(scheme.surfaceContainerHigh, CircleShape).padding(horizontal = 12.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(Lucide.BookOpenCheck, null, Modifier.size(12.dp), tint = scheme.onSurfaceVariant)
            Spacer(Modifier.width(Space.xs))
            Text(
                "$days ${if (days == 1) "day" else "days"} to prepare",
                style = MaterialTheme.typography.labelMedium,
                color = scheme.onSurfaceVariant,
            )
        }
    }
}
