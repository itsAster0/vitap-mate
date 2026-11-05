package vitapmate.features.timetable

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.drawscope.clipRect
import androidx.compose.ui.unit.dp
import vitapmate.core.ClassKind
import vitapmate.core.ClassSlot
import vitapmate.ui.AppTheme

/**
 * The day's shape on one line: a pill per class (lab or theory colour) along the span from the
 * first class to the last ([span] shares one scale across days), filled up to [minuteNow] (today
 * only) with a marker at it.
 */
@Composable
fun DayBar(slots: List<ClassSlot>, minuteNow: Int?, modifier: Modifier = Modifier, span: Pair<Int, Int>? = null) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val start = span?.first ?: slots.minOf { it.start }
    val end = span?.second ?: slots.maxOf { it.end }
    val length = (end - start).coerceIn(1, 24 * 60)
    Canvas(modifier.fillMaxWidth().height(12.dp)) {
        fun x(minute: Int) = ((minute - start) / length.toFloat()).coerceIn(0f, 1f) * size.width
        val h = 6.dp.toPx()
        val top = (size.height - h) / 2
        val r = CornerRadius(h / 2)
        drawRoundRect(scheme.surfaceContainerHighest, Offset(0f, top + h / 2 - 1.dp.toPx()), Size(size.width, 2.dp.toPx()), CornerRadius(1.dp.toPx()))
        for (slot in slots) {
            val tone = if (slot.kind == ClassKind.LAB) tones.lab else tones.accentTone
            val left = x(slot.start) + 1.dp.toPx()
            val width = maxOf(h, x(slot.end) - x(slot.start) - 2.dp.toPx())
            // Another day: the solid colour; today: the quiet fill, solid up to now.
            drawRoundRect(if (minuteNow == null) tone.base else tone.subtle, Offset(left, top), Size(width, h), r)
            if (minuteNow != null && minuteNow > slot.start) {
                val done = ((minuteNow - slot.start) / (slot.end - slot.start).toFloat()).coerceIn(0f, 1f)
                clipRect(right = left + width * done) { drawRoundRect(tone.base, Offset(left, top), Size(width, h), r) }
            }
        }
        if (minuteNow != null && minuteNow in start..end) {
            drawRoundRect(scheme.onSurface, Offset(x(minuteNow) - 1.5.dp.toPx(), 0f), Size(3.dp.toPx(), size.height), CornerRadius(1.5.dp.toPx()))
        }
    }
}
