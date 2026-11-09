package vitapmate.features.marks

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
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
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronDown
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sparkles
import vitapmate.core.MarksCourse
import vitapmate.ui.KindChip
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.nums
import vitapmate.ui.tappable

/**
 * One course: name, kind and code; the weighted total so far large on the right with its share;
 * the [WeightBar]; the latest mark posted and the weightage still to come. Tapping toggles
 * [expanded]: faculty and slot, every assessment, and the total of their weighted marks.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun MarksCard(course: MarksCourse, expanded: Boolean, onToggle: () -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val motion = MaterialTheme.motionScheme
    val name = course.name.ifBlank { course.code }
    val latest = course.latest?.let { course.assessments.getOrNull(it) }
    val assessed = course.maxWeighted > 0
    val chevron by animateFloatAsState(if (expanded) 180f else 0f, motion.defaultSpatialSpec())
    SectionCard(
        modifier.fillMaxWidth()
            .tappable(onToggle, name, Role.Button, pressedScale = 0.99f)
            .testTag("marks.course.${course.key}")
            .semantics {
                stateDescription = if (expanded) "expanded" else "collapsed"
                contentDescription = listOfNotNull(
                    name,
                    if (assessed) "${course.weighted.marks()} of ${course.maxWeighted.marks()}" else "no marks yet",
                    latest?.let { "latest ${it.title} ${it.scoreLabel}" },
                ).joinToString(", ")
            },
    ) {
        Row(verticalAlignment = Alignment.Top) {
            Column(Modifier.weight(1f)) {
                Text(name, style = MaterialTheme.typography.titleMedium, maxLines = 2, overflow = TextOverflow.Ellipsis)
                Spacer(Modifier.height(Space.xs))
                Row(horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
                    KindChip(course.isLab)
                    Text(course.code, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant, maxLines = 1)
                }
            }
            Spacer(Modifier.width(Space.md))
            Column(horizontalAlignment = Alignment.End) {
                Text(
                    buildAnnotatedString {
                        append(course.weighted.marks())
                        withStyle(SpanStyle(color = scheme.onSurfaceVariant)) { append(" / ${course.maxWeighted.marks()}") }
                    },
                    style = MaterialTheme.typography.headlineSmallEmphasized.nums(),
                    maxLines = 1,
                )
                course.percent?.let {
                    Spacer(Modifier.height(Space.xs))
                    StatusChip(it.percentLabel(), course.band.tone())
                }
            }
        }
        WeightBar(course, Modifier.padding(vertical = Space.md))
        Row(verticalAlignment = Alignment.CenterVertically) {
            if (latest != null) {
                Icon(Lucide.Sparkles, null, Modifier.size(14.dp), tint = latest.band.tone().base)
                Spacer(Modifier.width(Space.xs))
                Text(
                    "${latest.title} · ${latest.scoreLabel}",
                    Modifier.weight(1f),
                    style = MaterialTheme.typography.labelLarge.nums(),
                    color = scheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            } else {
                Text("No marks yet", Modifier.weight(1f), style = MaterialTheme.typography.labelLarge, color = scheme.onSurfaceVariant)
            }
            Text(
                if (course.toCome > 0) "${course.toCome.marks()} to come" else "All assessed",
                style = MaterialTheme.typography.labelMedium.nums(),
                color = scheme.onSurfaceVariant,
                maxLines = 1,
            )
            Spacer(Modifier.width(Space.xs))
            Icon(Lucide.ChevronDown, null, Modifier.size(16.dp).rotate(chevron), tint = scheme.onSurfaceVariant)
        }
        AnimatedVisibility(
            expanded,
            enter = expandVertically(motion.defaultSpatialSpec()) + fadeIn(motion.defaultEffectsSpec()),
            exit = shrinkVertically(motion.fastSpatialSpec()) + fadeOut(motion.fastEffectsSpec()),
        ) {
            Assessments(course)
        }
    }
}

/** Faculty and slot, a row per assessment, then their weighted total. */
@Composable
private fun Assessments(course: MarksCourse) {
    val scheme = MaterialTheme.colorScheme
    Column(Modifier.padding(top = Space.md)) {
        HorizontalDivider(color = scheme.outlineVariant)
        Text(
            listOf(course.faculty, course.slot).filter { it.isNotBlank() }.joinToString(" · "),
            Modifier.padding(top = Space.md),
            style = MaterialTheme.typography.labelLarge,
            color = scheme.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
        course.assessments.forEachIndexed { i, assessment ->
            if (i > 0) HorizontalDivider(color = scheme.outlineVariant.copy(alpha = 0.6f))
            AssessmentRow(assessment, latest = i == course.latest, Modifier.testTag("marks.course.${course.key}.assessment.$i"))
        }
        if (course.assessments.isNotEmpty()) {
            HorizontalDivider(color = scheme.outlineVariant)
            Row(Modifier.fillMaxWidth().padding(top = Space.md).testTag("marks.course.${course.key}.total"), verticalAlignment = Alignment.CenterVertically) {
                Text("Total", Modifier.weight(1f), style = MaterialTheme.typography.titleSmall)
                Text(
                    "${course.maxWeighted.marks()}% → ${course.weighted.marks()}",
                    style = MaterialTheme.typography.titleSmall.nums(),
                )
            }
        }
    }
}
