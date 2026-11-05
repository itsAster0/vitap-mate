package vitapmate.features.exams

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip
import vitapmate.ui.Space
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/** "MON 10 AUG", how far off (Today in danger, Tomorrow in warning, In 3 days in the accent), and the day's count. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun DayHeader(day: Int, today: Int, count: Int, modifier: Modifier = Modifier) {
    val tones = AppTheme.tones
    val scheme = MaterialTheme.colorScheme
    val (label, tone) = when (val days = day - today) {
        0 -> "Today" to tones.danger
        1 -> "Tomorrow" to tones.warning
        else -> "In $days days" to tones.accentTone
    }
    Row(
        modifier.fillMaxWidth().padding(start = 2.dp, top = Space.md, end = 2.dp, bottom = Space.xs).testTag("exams.day.$day"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(formatDay(day, "EEE d MMM").uppercase(), style = MaterialTheme.typography.titleSmallEmphasized.nums())
        Spacer(Modifier.width(Space.sm))
        StatusChip(label, tone)
        Spacer(Modifier.weight(1f))
        if (count > 1) Text("$count exams", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
    }
}
