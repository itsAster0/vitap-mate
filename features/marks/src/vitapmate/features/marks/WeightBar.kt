package vitapmate.features.marks

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.Canvas
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import vitapmate.core.MarksCourse
import vitapmate.core.ScoreBand

/**
 * The course's 100 weighted marks as one line: a segment per assessed mark, as wide as its
 * weightage and filled by what it scored, in its band's tone; then the weightage still to come.
 * The fills spring out from zero when first shown.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun WeightBar(course: MarksCourse, modifier: Modifier = Modifier, thickness: Dp = 8.dp) {
    val parts = course.assessments.mapNotNull { a ->
        val weight = a.weightage ?: return@mapNotNull null
        if (a.reevaluation || (a.scored == null && a.band != ScoreBand.ABSENT)) return@mapNotNull null
        Triple(weight.toFloat(), ((a.weighted ?: 0.0) / weight).toFloat().coerceIn(0f, 1f), a.band.tone())
    }
    var shown by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { shown = true }
    val grow by animateFloatAsState(if (shown) 1f else 0f, MaterialTheme.motionScheme.slowSpatialSpec())
    val rest = MaterialTheme.colorScheme.surfaceContainerHighest
    Canvas(modifier.fillMaxWidth().height(thickness).clip(RoundedCornerShape(thickness))) {
        val gap = 2.dp.toPx()
        val total = maxOf(100f, parts.sumOf { it.first.toDouble() }.toFloat())
        val room = size.width - gap * parts.size
        var x = 0f
        val radius = CornerRadius(1.dp.toPx())
        parts.forEach { (weight, fill, tone) ->
            val w = room * weight / total
            drawRoundRect(tone.base.copy(alpha = 0.25f), Offset(x, 0f), Size(w, size.height), radius)
            // The spring overshoots a little; never past the segment.
            drawRoundRect(tone.base, Offset(x, 0f), Size((w * fill * grow).coerceIn(0f, w), size.height), radius)
            x += w + gap
        }
        if (x < size.width) drawRoundRect(rest, Offset(x, 0f), Size(size.width - x, size.height), radius)
    }
}
