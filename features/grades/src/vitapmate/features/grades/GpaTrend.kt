package vitapmate.features.grades

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextMeasurer
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme
import vitapmate.ui.nums

/** Session GPAs as a line over a soft fill, a dot and value per session (the latest larger), and the session under each. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun GpaTrend(points: List<Pair<String, Double>>, modifier: Modifier = Modifier) {
    val accent = AppTheme.tones.accent
    // Rings the dots in the block's fill.
    val hole = AppTheme.tones.accentTone.subtle
    val valueStyle = MaterialTheme.typography.labelMedium.nums(fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.onSurface)
    val measurer: TextMeasurer = rememberTextMeasurer()
    var shown by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { shown = true }
    val rise by animateFloatAsState(if (shown) 1f else 0f, MaterialTheme.motionScheme.slowSpatialSpec())
    val lo = (points.minOf { it.second } - 0.5).coerceIn(0.0, 10.0)
    val hi = (points.maxOf { it.second } + 0.3).coerceIn(0.0, 10.0)
    Column(
        modifier.fillMaxWidth().testTag("grades.trend").semantics {
            contentDescription = points.joinToString(", ") { "${it.first} ${it.second.fixed2()}" }
        },
    ) {
        Canvas(Modifier.fillMaxWidth().height(96.dp)) {
            val top = 30.dp.toPx()
            val chart = size.height - top - 8.dp.toPx()
            val step = size.width / points.size
            val at = points.mapIndexed { i, (_, gpa) ->
                // Rises from the floor when first shown.
                val y = top + chart * (1 - ((gpa - lo) / (hi - lo)).toFloat() * rise)
                Offset(step * (i + 0.5f), y)
            }
            fun Path.through() = at.forEachIndexed { i, p -> if (i == 0) moveTo(p.x, p.y) else lineTo(p.x, p.y) }
            val line = Path().apply { through() }
            // The fill runs from the line down to the chart's floor.
            val area = Path().apply {
                through()
                lineTo(at.last().x, size.height)
                lineTo(at.first().x, size.height)
                close()
            }
            drawPath(area, accent.copy(alpha = 0.12f))
            drawPath(line, accent, style = Stroke(3.dp.toPx(), cap = StrokeCap.Round, join = StrokeJoin.Round))
            at.forEachIndexed { i, p ->
                val r = (if (i == at.lastIndex) 7 else 5).dp.toPx()
                drawCircle(hole, r + 1.5.dp.toPx(), p)
                drawCircle(accent, r, p)
                val label = measurer.measure(points[i].second.fixed2(), valueStyle)
                drawText(label, topLeft = Offset(p.x - label.size.width / 2f, p.y - r - 6.dp.toPx() - label.size.height))
            }
        }
        Row(Modifier.fillMaxWidth()) {
            points.forEach { (session, _) ->
                Text(session, Modifier.weight(1f), style = MaterialTheme.typography.labelSmall, color = AppTheme.tones.accentTone.onSubtle, textAlign = TextAlign.Center, maxLines = 1)
            }
        }
    }
}
