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
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.HorizontalDivider
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
import vitapmate.core.HistoryCourse
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.nums
import vitapmate.ui.tappable

/** One graded course: badge, title, code and category, credits; an embedded course expands to its parts. [shape] places it in its run. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun HistoryCourseCard(course: HistoryCourse, expanded: Boolean, onToggle: () -> Unit, modifier: Modifier = Modifier, shape: Shape = MaterialTheme.shapes.large) {
    val scheme = MaterialTheme.colorScheme
    val motion = MaterialTheme.motionScheme
    val parts = course.components.isNotEmpty()
    val chevron by animateFloatAsState(if (expanded) 180f else 0f, motion.defaultSpatialSpec())
    SectionCard(
        modifier.fillMaxWidth()
            .then(if (parts) Modifier.tappable(onToggle, course.title, Role.Button, pressedScale = 0.99f) else Modifier)
            .testTag("grades.history.${course.key}")
            .semantics {
                if (parts) stateDescription = if (expanded) "expanded" else "collapsed"
                contentDescription = "${course.title}, grade ${course.grade}" + (course.credits?.let { ", ${it.creditsLabel()} credits" } ?: "")
            },
        shape = shape,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            GradeBadge(course.grade)
            Spacer(Modifier.width(Space.md))
            Column(Modifier.weight(1f)) {
                Text(course.title, style = MaterialTheme.typography.titleMedium, maxLines = 2, overflow = TextOverflow.Ellipsis)
                Text(
                    listOf(course.code, course.distribution).filter { it.isNotBlank() }.joinToString(" · "),
                    style = MaterialTheme.typography.labelMedium,
                    color = scheme.onSurfaceVariant,
                    maxLines = 1,
                )
            }
            course.credits?.let {
                Spacer(Modifier.width(Space.md))
                Column(horizontalAlignment = Alignment.End) {
                    Text(it.creditsLabel(), style = MaterialTheme.typography.titleLargeEmphasized.nums())
                    Text(if (it == 1.0) "credit" else "credits", style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
                }
            }
            Spacer(Modifier.width(Space.sm))
            // Space kept without parts, so credits line up.
            if (parts) Icon(Lucide.ChevronDown, null, Modifier.size(16.dp).rotate(chevron), tint = scheme.onSurfaceVariant)
            else Spacer(Modifier.width(16.dp))
        }
        AnimatedVisibility(
            expanded && parts,
            enter = expandVertically(motion.defaultSpatialSpec()) + fadeIn(motion.defaultEffectsSpec()),
            exit = shrinkVertically(motion.fastSpatialSpec()) + fadeOut(motion.fastEffectsSpec()),
        ) {
            Column(Modifier.padding(top = Space.md)) {
                HorizontalDivider(color = scheme.outlineVariant)
                course.components.forEach { part ->
                    Row(Modifier.fillMaxWidth().padding(top = Space.sm), verticalAlignment = Alignment.CenterVertically) {
                        Text(part.label, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium)
                        if (part.declared.isNotBlank()) {
                            Text("Declared ${part.declared}", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
                        }
                    }
                }
            }
        }
    }
}
