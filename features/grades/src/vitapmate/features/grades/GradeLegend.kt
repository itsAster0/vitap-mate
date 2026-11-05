package vitapmate.features.grades

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import vitapmate.core.GradeCount
import vitapmate.ui.Space
import vitapmate.ui.nums

/** "■ S 3  ■ A 2": each grade's square in its tone, with its count. */
@Composable
internal fun GradeLegend(counts: List<GradeCount>, modifier: Modifier = Modifier) {
    FlowRow(modifier, horizontalArrangement = Arrangement.spacedBy(Space.md), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        counts.forEach { count ->
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.size(8.dp).background(gradeTone(count.grade).base, RoundedCornerShape(2.dp)))
                Spacer(Modifier.width(Space.xs))
                Text("${count.grade} ${count.count}", style = MaterialTheme.typography.labelMedium.nums(), color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}
