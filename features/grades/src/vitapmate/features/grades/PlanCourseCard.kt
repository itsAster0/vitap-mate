package vitapmate.features.grades

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Minus
import com.composables.icons.lucide.Plus
import com.composables.icons.lucide.X
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.IconButton
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.nums

/** Grades the planner offers (N carries no points and is left out). */
private val PICKER = listOf("S", "A", "B", "C", "D", "E", "F")

/** One planned course: name (or "Course n") and remove; code and a credits stepper; the grade as connected buttons. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun PlanCourseCard(
    index: Int,
    row: PlanRow,
    canRemove: Boolean,
    onCredits: (Double) -> Unit,
    onGrade: (String) -> Unit,
    onRemove: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val scheme = MaterialTheme.colorScheme
    SectionCard(modifier.fillMaxWidth().testTag("grades.plan.row.$index")) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(row.name.ifEmpty { "Course ${index + 1}" }, Modifier.weight(1f), style = MaterialTheme.typography.titleSmall, maxLines = 2, overflow = TextOverflow.Ellipsis)
            if (canRemove) IconButton(Lucide.X, onRemove, "Remove course", Modifier.testTag("grades.plan.row.$index.remove"))
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(row.code, Modifier.weight(1f), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
            IconButton(Lucide.Minus, if (row.credits > 0.5) ({ onCredits((row.credits - 0.5).coerceIn(0.5, 30.0)) }) else null, "Fewer credits", Modifier.testTag("grades.plan.row.$index.less"))
            Text("${row.credits.creditsLabel()} cr", Modifier.width(56.dp), style = MaterialTheme.typography.titleSmall.nums(), textAlign = TextAlign.Center)
            IconButton(Lucide.Plus, { onCredits((row.credits + 0.5).coerceIn(0.5, 30.0)) }, "More credits", Modifier.testTag("grades.plan.row.$index.more"))
        }
        Spacer(Modifier.height(Space.sm))
        ConnectedChoice(
            PICKER,
            row.grade,
            onSelect = onGrade,
            tag = "grades.plan.row.$index.grade",
            key = { it },
            label = { it },
        )
    }
}
