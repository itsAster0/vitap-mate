package vitapmate.features.grades

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronDown
import com.composables.icons.lucide.Lucide
import vitapmate.core.GradeCourse
import vitapmate.core.GradeDetails
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.nums
import vitapmate.ui.tappable

/** One course's grade: badge, title, code and type, VTOP's total; tapping toggles its [GradeDetailsPanel]. [shape] places it in its run. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun GradeCard(
    course: GradeCourse,
    expanded: Boolean,
    details: GradeDetails?,
    loading: Boolean,
    failed: Boolean,
    onToggle: () -> Unit,
    onRefresh: () -> Unit,
    modifier: Modifier = Modifier,
    shape: Shape = MaterialTheme.shapes.large,
) {
    val scheme = MaterialTheme.colorScheme
    val motion = MaterialTheme.motionScheme
    val chevron by animateFloatAsState(if (expanded) 180f else 0f, motion.defaultSpatialSpec())
    val title = course.title.ifBlank { course.code }
    SectionCard(
        modifier.fillMaxWidth()
            .tappable(onToggle, title, Role.Button, pressedScale = 0.99f)
            .testTag("grades.course.${course.courseId}")
            .semantics {
                stateDescription = if (expanded) "expanded" else "collapsed"
                contentDescription = "$title, grade ${course.grade.ifEmpty { "none" }}, total ${course.total}"
            },
        shape = shape,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            GradeBadge(course.grade)
            Spacer(Modifier.width(Space.md))
            Column(Modifier.weight(1f)) {
                Text(title, style = MaterialTheme.typography.titleMedium, maxLines = 2, overflow = TextOverflow.Ellipsis)
                Text(
                    listOf(course.code, course.courseType).filter { it.isNotBlank() }.joinToString(" · "),
                    style = MaterialTheme.typography.labelMedium,
                    color = scheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Spacer(Modifier.width(Space.md))
            Column(horizontalAlignment = Alignment.End) {
                Text(course.total.ifEmpty { "–" }, style = MaterialTheme.typography.titleLargeEmphasized.nums(), maxLines = 1)
                Text("total", style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
            }
            Spacer(Modifier.width(Space.xs))
            Icon(Lucide.ChevronDown, null, Modifier.size(16.dp).rotate(chevron), tint = scheme.onSurfaceVariant)
        }
        AnimatedVisibility(
            expanded,
            enter = expandVertically(motion.defaultSpatialSpec()) + fadeIn(motion.defaultEffectsSpec()),
            exit = shrinkVertically(motion.fastSpatialSpec()) + fadeOut(motion.fastEffectsSpec()),
        ) {
            GradeDetailsPanel(course, details, loading, failed, onRefresh)
        }
    }
}
