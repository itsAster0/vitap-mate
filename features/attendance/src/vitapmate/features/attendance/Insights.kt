package vitapmate.features.attendance

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.CircleShape
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import vitapmate.core.AttendedClass
import vitapmate.core.ClassStatus
import vitapmate.ui.AppTheme
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.Tone
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/**
 * The history at a glance: one bar split into present / on duty / absent (it springs in), the
 * counts in their tones, and "5 in a row · last missed 3 Oct" (or "never missed"). Classes not
 * posted yet are left out. [classes] are newest first.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun Insights(classes: List<AttendedClass>) {
    val posted = classes.filter { it.status != null }
    if (posted.isEmpty()) return
    val tones = AppTheme.tones
    val present = posted.count { it.status == ClassStatus.PRESENT }
    val onDuty = posted.count { it.status == ClassStatus.ON_DUTY }
    val absent = posted.count { it.status == ClassStatus.ABSENT }
    val streak = posted.takeWhile { it.status == ClassStatus.PRESENT || it.status == ClassStatus.ON_DUTY }.size
    val lastMissed = posted.firstOrNull { it.status == ClassStatus.ABSENT }
    val note = listOfNotNull(
        "$streak in a row".takeIf { streak >= 3 },
        lastMissed?.let { "last missed ${formatDay(it.day, "d MMM")}" } ?: "never missed",
    ).joinToString(" · ").replaceFirstChar { it.uppercase() }
    var shown by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { shown = true }
    val grow by animateFloatAsState(if (shown) 1f else 0f, MaterialTheme.motionScheme.slowSpatialSpec())
    SectionCard(Modifier.fillMaxWidth().testTag("attendance.detail.insights")) {
        Box(Modifier.fillMaxWidth().height(10.dp)) {
            Row(
                Modifier.fillMaxWidth(grow.coerceIn(0.01f, 1f)).fillMaxHeight().clip(CircleShape),
                horizontalArrangement = Arrangement.spacedBy(2.dp),
            ) {
                for ((count, color) in listOf(present to tones.success.base, onDuty to tones.accentTone.base, absent to tones.danger.base)) {
                    if (count > 0) Box(Modifier.weight(count.toFloat()).fillMaxHeight().background(color))
                }
            }
        }
        Row(Modifier.padding(top = Space.md)) {
            Counts(Modifier.weight(1f), present to "present" to tones.success, onDuty to "on duty" to tones.accentTone)
            Counts(Modifier, absent to "absent" to tones.danger)
        }
        Text(note, Modifier.padding(top = Space.xs), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}

/** "12 present  ·  1 on duty", each number bold in its tone. */
@Composable
private fun Counts(modifier: Modifier, vararg parts: Pair<Pair<Int, String>, Tone>) = Text(
    buildAnnotatedString {
        parts.forEachIndexed { i, (part, tone) ->
            if (i > 0) append("  ·  ")
            withStyle(SpanStyle(fontWeight = FontWeight.Bold, color = tone.onSubtle)) { append("${part.first}") }
            append(" ${part.second}")
        }
    },
    modifier,
    style = MaterialTheme.typography.bodyMedium.nums(),
    color = MaterialTheme.colorScheme.onSurfaceVariant,
)
