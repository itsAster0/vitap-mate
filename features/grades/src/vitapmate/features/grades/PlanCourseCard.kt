package vitapmate.features.grades

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.ToggleButton
import androidx.compose.material3.ToggleButtonDefaults
import androidx.compose.material3.ToggleButtonShapes
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Minus
import com.composables.icons.lucide.Plus
import com.composables.icons.lucide.X
import vitapmate.ui.AppTheme
import vitapmate.ui.IconButton
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.nums
import vitapmate.ui.rememberHaptics

/** Grades the planner offers (N carries no points and is left out). */
private val PICKER = listOf("S", "A", "B", "C", "D", "E", "F")

/**
 * One planned course: name (or "Course n") and remove; code and a credits stepper; the grade as
 * connected buttons, the picked one wider, fully round and in its grade's tone (each tagged
 * `grades.plan.row.<index>.grade.<grade>`). [shape] places it in its run.
 */
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
    shape: Shape = MaterialTheme.shapes.large,
) {
    val scheme = MaterialTheme.colorScheme
    SectionCard(modifier.fillMaxWidth().testTag("grades.plan.row.$index"), shape = shape) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(row.name.ifEmpty { "Course ${index + 1}" }, Modifier.weight(1f), style = MaterialTheme.typography.titleSmall, maxLines = 2, overflow = TextOverflow.Ellipsis)
            if (canRemove) IconButton(Lucide.X, onRemove, "Remove course", Modifier.testTag("grades.plan.row.$index.remove"))
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(row.code, Modifier.weight(1f), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
            StepButton(Lucide.Minus, if (row.credits > 0.5) ({ onCredits((row.credits - 0.5).coerceIn(0.5, 30.0)) }) else null, "Fewer credits", Modifier.testTag("grades.plan.row.$index.less"))
            Text("${row.credits.creditsLabel()} cr", Modifier.width(58.dp), style = MaterialTheme.typography.titleSmall.nums(fontWeight = FontWeight.Bold), textAlign = TextAlign.Center)
            StepButton(Lucide.Plus, { onCredits((row.credits + 0.5).coerceIn(0.5, 30.0)) }, "More credits", Modifier.testTag("grades.plan.row.$index.more"))
        }
        Spacer(Modifier.height(Space.md))
        val haptics = rememberHaptics()
        val onFill = AppTheme.colors.background
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(3.dp)) {
            PICKER.forEachIndexed { i, grade ->
                val on = grade == row.grade
                val tone = gradeTone(grade)
                val weight by animateFloatAsState(if (on) 1.5f else 1f, MaterialTheme.motionScheme.fastSpatialSpec(), label = "grade.weight")
                ToggleButton(
                    checked = on,
                    onCheckedChange = { haptics.tick(); onGrade(grade) },
                    modifier = Modifier.weight(weight).height(40.dp).semantics { role = Role.RadioButton }.testTag("grades.plan.row.$index.grade.$grade"),
                    shapes = ToggleButtonShapes(connected(i, PICKER.size, 20.dp, 8.dp), connected(i, PICKER.size, 12.dp, 4.dp), RoundedCornerShape(20.dp)),
                    colors = ToggleButtonDefaults.toggleButtonColors(
                        containerColor = scheme.surfaceContainerHigh,
                        contentColor = scheme.onSurfaceVariant,
                        checkedContainerColor = tone.base,
                        checkedContentColor = onFill,
                    ),
                    contentPadding = PaddingValues(0.dp),
                ) {
                    Text(grade, maxLines = 1, softWrap = false, style = MaterialTheme.typography.labelLarge, fontWeight = if (on) FontWeight.ExtraBold else FontWeight.SemiBold)
                }
            }
        }
    }
}
