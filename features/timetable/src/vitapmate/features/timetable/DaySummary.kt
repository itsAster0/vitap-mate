package vitapmate.features.timetable

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import kotlin.math.PI
import vitapmate.core.ClassStatus
import vitapmate.core.DayClass
import vitapmate.ui.AppTheme
import vitapmate.ui.nums

/**
 * The card once the day's classes are over: a ring with one arc per class, clockwise from the top
 * in class order, coloured by its posted attendance (dim until VTOP posts it, amber for a Lab
 * FAT), beside the counts.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun DaySummary(classes: List<DayClass>, labFats: Set<DayClass>, isToday: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val pending = scheme.surfaceContainerHighest
    val arcs = classes.map { c ->
        if (c in labFats) tones.warning.base else when (c.posted) {
            null -> pending
            ClassStatus.PRESENT -> tones.success.base
            ClassStatus.ON_DUTY -> tones.accent
            ClassStatus.ABSENT -> tones.danger.base
            ClassStatus.OTHER -> scheme.onSurfaceVariant
        }
    }
    val graded = classes.filter { it !in labFats }.map { it.posted }
    fun count(s: ClassStatus?) = graded.count { it == s }
    val legend = listOf(
        Triple(count(ClassStatus.PRESENT), "present", tones.success.base),
        Triple(count(ClassStatus.ON_DUTY), "on duty", tones.accent),
        Triple(count(ClassStatus.ABSENT), "absent", tones.danger.base),
        Triple(labFats.size, if (labFats.size == 1) "Lab FAT" else "Lab FATs", tones.warning.base),
        Triple(count(null), "not posted yet", pending),
    ).filter { it.first > 0 }

    Card(
        Modifier.fillMaxWidth().testTag("timetable.summary"),
        shape = MaterialTheme.shapes.extraLargeIncreased,
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Row(Modifier.padding(20.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(
                Modifier.size(92.dp).clearAndSetSemantics {
                    contentDescription = classes.joinToString("; ") { c ->
                        "${c.slot.name}, ${if (c in labFats) "Lab FAT" else c.posted?.name?.lowercase()?.replace('_', ' ') ?: "not posted"}"
                    }
                },
                contentAlignment = Alignment.Center,
            ) {
                Canvas(Modifier.fillMaxSize()) {
                    if (arcs.isEmpty()) return@Canvas
                    val stroke = 8.dp.toPx()
                    val radius = (size.minDimension - stroke) / 2
                    // Round caps reach past each arc's end; the gap leaves room for both.
                    val gap = if (arcs.size == 1) 0f else stroke / radius + 0.14f
                    val sweep = (2 * PI.toFloat() - gap * arcs.size) / arcs.size
                    val toDeg = 180f / PI.toFloat()
                    arcs.forEachIndexed { i, color ->
                        drawArc(
                            color,
                            (-PI.toFloat() / 2 + gap / 2 + i * (sweep + gap)) * toDeg,
                            sweep * toDeg,
                            false,
                            topLeft = Offset(center.x - radius, center.y - radius),
                            size = Size(radius * 2, radius * 2),
                            style = Stroke(stroke, cap = if (arcs.size == 1) StrokeCap.Butt else StrokeCap.Round),
                        )
                    }
                }
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    Text("${classes.size}", style = MaterialTheme.typography.titleLargeEmphasized.nums())
                    Text(if (classes.size == 1) "class" else "classes", style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
                }
            }
            Spacer(Modifier.width(20.dp))
            Column(Modifier.weight(1f)) {
                Text(if (isToday) "Done for today" else "Classes done", style = MaterialTheme.typography.titleLargeEmphasized)
                Spacer(Modifier.height(6.dp))
                FlowRow(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    for ((n, label, color) in legend) {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Box(Modifier.size(8.dp).background(color, CircleShape))
                            Spacer(Modifier.width(6.dp))
                            Text(
                                buildAnnotatedString {
                                    withStyle(SpanStyle(color = scheme.onSurface)) { append("$n") }
                                    append(" $label")
                                },
                                style = MaterialTheme.typography.labelLarge.nums(),
                                color = scheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        }
    }
}
