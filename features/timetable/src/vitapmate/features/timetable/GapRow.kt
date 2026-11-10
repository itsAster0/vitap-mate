package vitapmate.features.timetable

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.PathEffect
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
import vitapmate.ui.clock
import vitapmate.ui.compactMinutes
import vitapmate.ui.nums

/**
 * "Free until 3:50 PM · 1h 10m" between two groups of classes, on the timeline's rail (dotted, in
 * the accent once [done]) with a cup as its node; "· 40m left" and primary while it's [isNow].
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun GapRow(previousEnd: Int, nextStart: Int, isNow: Boolean, done: Boolean, minuteNow: Int, h24: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val color = if (isNow) scheme.primary else scheme.onSurfaceVariant
    val rail = if (done) AppTheme.tones.accent else scheme.outlineVariant
    val length = if (isNow) "${compactMinutes(nextStart - minuteNow)} left" else compactMinutes(nextStart - previousEnd)
    val timeWidth = timeColumnWidth(h24)
    Row(
        Modifier
            .fillMaxWidth()
            .drawBehind {
                val x = (timeWidth + RailGap + RailX).toPx()
                val dash = PathEffect.dashPathEffect(floatArrayOf(3.dp.toPx(), 4.dp.toPx()))
                drawLine(rail, Offset(x, 0f), Offset(x, size.height), 2.dp.toPx(), pathEffect = dash)
            }
            .padding(start = timeWidth + RailGap, end = 16.dp)
            .testTag("timetable.gap.$previousEnd"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(Rail).padding(vertical = 9.dp)) {
            Box(Modifier.offset(x = RailX - 10.dp).size(20.dp).background(scheme.surface, CircleShape), Alignment.Center) {
                Icon(Lucide.Coffee, null, Modifier.size(14.dp), tint = color)
            }
        }
        Spacer(Modifier.width(14.dp))
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
    }
}
