package vitapmate.features.grades

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import vitapmate.core.GradeCount
import vitapmate.ui.AppTheme
import vitapmate.ui.Space

/** One bar of the grade mix (a segment per grade, as wide as its count, lettered when it fits), then a legend. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun GradeMix(counts: List<GradeCount>, modifier: Modifier = Modifier) {
    if (counts.isEmpty()) return
    val tones = counts.map { gradeTone(it.grade).base }
    val letter = MaterialTheme.typography.labelLargeEmphasized.copy(color = AppTheme.colors.background)
    val measurer = rememberTextMeasurer()
    var shown by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { shown = true }
    val grow by animateFloatAsState(if (shown) 1f else 0f, MaterialTheme.motionScheme.slowSpatialSpec())
    Column(modifier.fillMaxWidth().testTag("grades.mix")) {
        Canvas(Modifier.fillMaxWidth().height(30.dp)) {
            val gap = 3.dp.toPx()
            val outer = CornerRadius(15.dp.toPx())
            val inner = CornerRadius(9.dp.toPx())
            val total = counts.sumOf { it.count }.toFloat()
            val room = (size.width - gap * (counts.size - 1)) * grow.coerceIn(0f, 1f)
            var x = 0f
            counts.forEachIndexed { i, count ->
                val w = room * count.count / total
                val start = if (i == 0) outer else inner
                val end = if (i == counts.lastIndex) outer else inner
                drawPath(Path().apply { addRoundRect(RoundRect(x, 0f, x + w, size.height, start, end, end, start)) }, tones[i])
                val text = measurer.measure(count.grade, letter)
                if (text.size.width + 8.dp.toPx() <= w) {
                    drawText(text, topLeft = Offset(x + (w - text.size.width) / 2f, (size.height - text.size.height) / 2f))
                }
                x += w + gap
            }
        }
        Spacer(Modifier.height(Space.sm))
        GradeLegend(counts)
    }
}
