package vitapmate.features.exams

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import vitapmate.core.ScheduledPaper
import vitapmate.ui.Space

/** A paper VTOP has not dated yet: course name and code. */
@Composable
internal fun CourseLine(paper: ScheduledPaper, modifier: Modifier = Modifier) {
    Row(modifier.fillMaxWidth().testTag("exams.undated.${paper.key}"), verticalAlignment = Alignment.CenterVertically) {
        Text(paper.name.ifBlank { paper.code }, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
        Spacer(Modifier.width(Space.md))
        Text(paper.code, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
