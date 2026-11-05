package vitapmate.features.timetable

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Coffee
import com.composables.icons.lucide.Lucide
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip
import vitapmate.ui.clock
import vitapmate.ui.compactMinutes
import vitapmate.ui.nums

/** "Free until 3:50 PM · 1h 10m" between two groups of classes; "· 40m left" and primary while it's [isNow]. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun GapRow(previousEnd: Int, nextStart: Int, isNow: Boolean, minuteNow: Int, isLongest: Boolean, h24: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val color = if (isNow) scheme.primary else scheme.onSurfaceVariant
    val length = if (isNow) "${compactMinutes(nextStart - minuteNow)} left" else compactMinutes(nextStart - previousEnd)
    Row(
        Modifier.fillMaxWidth().padding(start = 28.dp, top = 10.dp, end = 16.dp, bottom = 10.dp).testTag("timetable.gap.$previousEnd"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(Lucide.Coffee, null, Modifier.size(16.dp), tint = color)
        Spacer(Modifier.width(10.dp))
        Text(
            buildAnnotatedString {
                withStyle(SpanStyle(fontWeight = FontWeight.W600, color = if (isNow) color else scheme.onSurface)) { append("Free until ${clock(nextStart, h24)}") }
                append("  ·  $length")
            },
            Modifier.weight(1f, fill = false),
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
            style = MaterialTheme.typography.bodyMedium.nums(),
            color = color,
        )
        if (isLongest && !isNow) {
            Spacer(Modifier.width(8.dp))
            StatusChip("Longest", AppTheme.tones.accentTone)
        }
    }
}
