package vitapmate.features.attendance

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import kotlin.coroutines.cancellation.CancellationException
import kotlin.math.floor
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendancePlan
import vitapmate.data.AppCore
import vitapmate.observability.flow
import vitapmate.ui.Space
import vitapmate.ui.Tone
import vitapmate.ui.nums

/**
 * Each course's plan by key (until the next exam if one comes first, else the semester's end),
 * from Rust's saved attendance, class histories, timetable and calendar; read again when [key]
 * (the attendance and the histories that arrived) changes. Empty without a calendar.
 */
@Composable
internal fun rememberOutlook(key: Any?): Map<String, AttendancePlan> {
    val outlook by produceState(emptyMap(), key) {
        value = withContext(Dispatchers.IO) {
            runCatching { flow("attendance.outlook") { id -> AppCore.await().attendanceOutlook(id) } }
                .onFailure { if (it is CancellationException) throw it }.getOrDefault(emptyList())
        }.associate { it.key to it.plan }
    }
    return outlook
}

/**
 * The outlook as connected cells: "1+16" over "left till CAT-2" (the muted "1+" is held but not
 * posted yet, then the classes to come, till the exam when the plan stops at one), then "8" over
 * "to spare" ("N needed" below 75%, or an alert cell in [tone] reading "short of 75%" when even all
 * of them are not enough), then "95%" over "max". Without [showSkip] the middle cell is left out.
 * No classes left: one cell saying so. Labs count in sessions. TalkBack reads it as one sentence.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun OutlookLine(course: AttendanceCourse, plan: AttendancePlan, showSkip: Boolean, tone: Tone, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val left = plan.upcoming + plan.unposted
    val must = plan.mustAttend
    val until = if (plan.until.isEmpty()) "left" else "left till ${plan.until}"
    val spare = when {
        !showSkip -> null
        must == null -> null to "short of 75%"
        !course.safe -> "$must" to "needed"
        else -> "${plan.canMiss}" to "to spare"
    }
    val best = "${floor(plan.bestPercent).toInt()}%"
    val unposted = if (plan.unposted > 0) "${plan.unposted}+" else ""
    // What TalkBack reads: the sentence the one-line outlook used to show.
    val sentence = when {
        left > 0 -> listOfNotNull(
            "$unposted${plan.upcoming} $until",
            spare?.let { (n, what) -> listOfNotNull(n, what).joinToString(" ") },
            "$best max",
        ).joinToString("  ·  ")
        plan.until.isEmpty() -> "No classes left this semester"
        else -> "No classes before ${plan.until}"
    }
    Row(
        modifier.fillMaxWidth().height(IntrinsicSize.Min).testTag("attendance.course.${course.key}.outlook").clearAndSetSemantics { contentDescription = sentence },
        horizontalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        if (left == 0) {
            Cell(Modifier.weight(1f), RoundedCornerShape(16.dp), scheme.surfaceContainer) {
                Text(sentence, style = MaterialTheme.typography.labelLarge.nums(), color = scheme.onSurfaceVariant, textAlign = TextAlign.Center)
            }
            return@Row
        }
        val count = if (spare != null) 3 else 2
        Cell(Modifier.weight(1.35f), connected(0, count, 16.dp, 6.dp), scheme.surfaceContainer) {
            Value(
                buildAnnotatedString {
                    withStyle(SpanStyle(fontWeight = FontWeight.Medium, color = scheme.onSurfaceVariant)) { append(unposted) }
                    append("${plan.upcoming}")
                },
                scheme.onSurface,
            )
            Key(until, scheme.onSurfaceVariant)
        }
        spare?.let { (n, what) ->
            val alert = n == null
            Cell(Modifier.weight(1f), connected(1, count, 16.dp, 6.dp), if (alert) tone.subtle else scheme.surfaceContainer) {
                if (alert) Value(AnnotatedString(what), tone.onSubtle, MaterialTheme.typography.labelLargeEmphasized)
                else {
                    Value(AnnotatedString(n), scheme.onSurface)
                    Key(what, scheme.onSurfaceVariant)
                }
            }
        }
        Cell(Modifier.weight(1f), connected(count - 1, count, 16.dp, 6.dp), scheme.surfaceContainer) {
            Value(AnnotatedString(best), scheme.onSurface)
            Key("max", scheme.onSurfaceVariant)
        }
    }
}

/** One outlook cell: its content centred on [fill]. */
@Composable
private fun Cell(modifier: Modifier, shape: Shape, fill: Color, content: @Composable ColumnScope.() -> Unit) = Column(
    modifier.fillMaxHeight().background(fill, shape).padding(horizontal = Space.xs, vertical = Space.sm),
    verticalArrangement = Arrangement.Center,
    horizontalAlignment = Alignment.CenterHorizontally,
    content = content,
)

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Value(text: AnnotatedString, color: Color, style: TextStyle = MaterialTheme.typography.titleSmallEmphasized.copy(fontWeight = FontWeight.Bold)) =
    Text(text, style = style.nums(), color = color, maxLines = 1, overflow = TextOverflow.Ellipsis, textAlign = TextAlign.Center)

@Composable
private fun Key(text: String, color: Color) =
    Text(text, style = MaterialTheme.typography.labelSmall, color = color, maxLines = 1, overflow = TextOverflow.Ellipsis, textAlign = TextAlign.Center)
