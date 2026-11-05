package vitapmate.features.grades

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.core.GradeCount
import vitapmate.ui.Space
import vitapmate.ui.nums

/** One bar of the grade mix (a segment per grade, as wide as its count), then a legend. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun GradeMix(counts: List<GradeCount>, modifier: Modifier = Modifier) {
    if (counts.isEmpty()) return
    val tones = counts.map { gradeTone(it.grade).base }
    var shown by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { shown = true }
    val grow by animateFloatAsState(if (shown) 1f else 0f, MaterialTheme.motionScheme.slowSpatialSpec())
    Column(modifier.fillMaxWidth().testTag("grades.mix")) {
        Canvas(Modifier.fillMaxWidth().height(10.dp).clip(RoundedCornerShape(5.dp))) {
            val gap = 2.dp.toPx()
            val total = counts.sumOf { it.count }.toFloat()
            val room = (size.width - gap * (counts.size - 1)) * grow.coerceIn(0f, 1f)
            var x = 0f
            counts.forEachIndexed { i, count ->
                val w = room * count.count / total
                drawRoundRect(tones[i], Offset(x, 0f), Size(w, size.height), CornerRadius(1.dp.toPx()))
                x += w + gap
            }
        }
        Spacer(Modifier.height(Space.sm))
        GradeLegend(counts)
    }
}
