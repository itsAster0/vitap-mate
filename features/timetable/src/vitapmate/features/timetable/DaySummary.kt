package vitapmate.features.timetable

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.min
import androidx.compose.ui.unit.sp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Ellipsis
import com.composables.icons.lucide.Flag
import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Minus
import com.composables.icons.lucide.X
import vitapmate.core.ClassStatus
import vitapmate.core.DayClass
import vitapmate.ui.AppTheme
import vitapmate.ui.nums

/**
 * The card once the day's classes are over: the title beside the class count in a cookie, a track
 * with one capsule per class in class order, coloured and marked by its posted attendance (dim
 * until VTOP posts it, amber for a Lab FAT), and the counts as chips.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun DaySummary(classes: List<DayClass>, labFats: Set<DayClass>, isToday: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val pending = scheme.surfaceContainerHighest
    // Fill and icon per class; the icon is on the surface colour (dark on light tones in dark mode).
    val marks = classes.map { c ->
        if (c in labFats) Triple(tones.warning.base, Lucide.FlaskConical, scheme.surface) else when (c.posted) {
            null -> Triple(pending, Lucide.Ellipsis, scheme.onSurfaceVariant)
            ClassStatus.PRESENT -> Triple(tones.success.base, Lucide.Check, scheme.surface)
            ClassStatus.ON_DUTY -> Triple(tones.accent, Lucide.Flag, scheme.surface)
            ClassStatus.ABSENT -> Triple(tones.danger.base, Lucide.X, scheme.surface)
            ClassStatus.OTHER -> Triple(scheme.onSurfaceVariant, Lucide.Minus, scheme.surface)
        }
    }
    val graded = classes.filter { it !in labFats }.map { it.posted }
    fun count(s: ClassStatus?) = graded.count { it == s }
    val neutral = scheme.surfaceContainerHigh to scheme.onSurfaceVariant
    val legend = listOf(
        Triple(count(ClassStatus.PRESENT), "present", tones.success.let { it.subtle to it.onSubtle }),
        Triple(count(ClassStatus.ON_DUTY), "on duty", tones.accentTone.let { it.subtle to it.onSubtle }),
        Triple(count(ClassStatus.ABSENT), "absent", tones.danger.let { it.subtle to it.onSubtle }),
        Triple(labFats.size, if (labFats.size == 1) "Lab FAT" else "Lab FATs", tones.warning.let { it.subtle to it.onSubtle }),
        Triple(count(null), "not posted yet", neutral),
    ).filter { it.first > 0 }

    Card(
        Modifier.fillMaxWidth().testTag("timetable.summary"),
        shape = MaterialTheme.shapes.extraLargeIncreased,
        colors = CardDefaults.cardColors(containerColor = scheme.surfaceContainerLow),
    ) {
        Column(Modifier.padding(horizontal = 18.dp, vertical = 20.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    if (isToday) "Done\nfor today" else "Classes\ndone",
                    Modifier.weight(1f),
                    style = MaterialTheme.typography.displaySmallEmphasized.copy(lineHeight = 1.04.em),
                )
                Box(Modifier.size(78.dp).background(scheme.surfaceContainerHigh, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Text("${classes.size}", style = MaterialTheme.typography.headlineLargeEmphasized.nums(fontSize = 28.sp, fontWeight = FontWeight.ExtraBold, lineHeight = 1.em))
                        Text(if (classes.size == 1) "class" else "classes", style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
                    }
                }
            }
            if (marks.isNotEmpty()) BoxWithConstraints(Modifier.padding(top = 16.dp, bottom = 14.dp)) {
                // Narrower capsules (8+ classes on a small phone) take a smaller gap and icon.
                val gap = if (maxWidth / marks.size < 32.dp) 2.dp else 3.dp
                val icon = min(18.dp, maxWidth / marks.size - gap - 8.dp)
                Row(
                    Modifier.fillMaxWidth().clearAndSetSemantics {
                        contentDescription = classes.joinToString("; ") { c ->
                            "${c.slot.name}, ${if (c in labFats) "Lab FAT" else c.posted?.name?.lowercase()?.replace('_', ' ') ?: "not posted"}"
                        }
                    },
                    horizontalArrangement = Arrangement.spacedBy(gap),
                ) {
                    marks.forEachIndexed { i, (fill, glyph, tint) ->
                        val start = if (i == 0) 23.dp else 8.dp
                        val end = if (i == marks.lastIndex) 23.dp else 8.dp
                        Box(Modifier.weight(1f).height(46.dp).background(fill, RoundedCornerShape(start, end, end, start)), Alignment.Center) {
                            if (icon > 8.dp) Icon(glyph, null, Modifier.size(icon), tint = tint)
                        }
                    }
                }
            }
            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                for ((n, label, colors) in legend) {
                    Text(
                        buildAnnotatedString {
                            withStyle(SpanStyle(fontWeight = FontWeight.ExtraBold, fontSize = 14.sp)) { append("$n") }
                            append(" $label")
                        },
                        Modifier.background(colors.first, CircleShape).padding(start = 10.dp, end = 12.dp, top = 6.dp, bottom = 6.dp),
                        style = MaterialTheme.typography.labelLarge.nums(fontSize = 12.5.sp),
                        color = colors.second,
                    )
                }
            }
        }
    }
}
