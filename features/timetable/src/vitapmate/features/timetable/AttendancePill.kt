package vitapmate.features.timetable

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.unit.dp
import kotlin.math.roundToInt
import vitapmate.core.AttendanceCourse
import vitapmate.data.Settings
import vitapmate.ui.AppTheme
import vitapmate.ui.Tone
import vitapmate.ui.nums

/** Red below 75%, amber when no class can be missed, green otherwise. */
@Composable
@ReadOnlyComposable
fun toneOf(c: AttendanceCourse): Tone = AppTheme.tones.run { if (!c.safe) danger else if (c.canSkip == 0) warning else success }

/** "need 2", "don't skip" or "skip 3" (in sessions for labs). */
fun skipHint(c: AttendanceCourse): String = when {
    !c.safe -> "need ${c.mustAttend}"
    c.canSkip == 0 -> "don't skip"
    else -> "skip ${c.canSkip}"
}

/**
 * A small ring filled to the course's attendance with "84%", then the skip hint ("skip 3") in a
 * smaller muted style when the setting is on and [hint], all on one line.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun AttendancePill(record: AttendanceCourse, modifier: Modifier = Modifier, hint: Boolean = true) {
    val tone = toneOf(record)
    val percent = "${record.percent.roundToInt()}%"
    val skips = hint && Settings.showSkipAdvice
    Row(
        modifier.clearAndSetSemantics { contentDescription = "$percent attendance${if (skips) ", ${skipHint(record)}" else ""}" },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        CircularProgressIndicator(
            progress = { (record.percent / 100).toFloat() },
            modifier = Modifier.size(14.dp),
            color = tone.base,
            trackColor = tone.subtle,
            strokeWidth = 2.5.dp,
            gapSize = 0.dp,
        )
        Spacer(Modifier.width(6.dp))
        Text(percent, style = MaterialTheme.typography.labelMediumEmphasized.nums(), color = tone.onSubtle, maxLines = 1)
        if (skips) {
            Spacer(Modifier.width(6.dp))
            Text(skipHint(record), style = MaterialTheme.typography.labelSmall.nums(), color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, softWrap = false)
        }
    }
}
