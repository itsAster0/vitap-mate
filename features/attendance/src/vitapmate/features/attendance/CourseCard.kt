package vitapmate.features.attendance

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import vitapmate.core.AttendanceCourse
import vitapmate.core.AttendancePlan
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.nums
import vitapmate.ui.tappable

/**
 * One course: name (two lines at most), kind and code, VTOP's percentage large on the right, the
 * progress line (tick at 75%) in the standing's tone, attended/held with the advice, and when it
 * meets [next], then the [plan]'s outlook line. Debarred: the percentage, the advice and a
 * one-line note in the danger colour.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun CourseCard(
    course: AttendanceCourse,
    plan: AttendancePlan?,
    showSkipAdvice: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val scheme = MaterialTheme.colorScheme
    val tone = course.standing.tone()
    val danger = course.debarred
    // The chip is what VTOP's count allows right now; the outlook line counts till the exam or the end.
    val advice = course.advice(showSkipAdvice)
    val name = course.name.ifBlank { course.code }
    val (attended, held) = course.sessions
    val debarLine = course.debarLine()
    SectionCard(
        modifier.fillMaxWidth()
            .tappable(onClick, name, Role.Button, pressedScale = 0.98f)
            .testTag("attendance.course.${course.key}")
            .semantics {
                contentDescription = listOfNotNull(
                    name,
                    if (course.total == 0) "no classes yet" else "${course.percentLabel}, $attended of $held attended",
                    debarLine,
                    advice,
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
            Text(
                course.percentLabel,
                style = MaterialTheme.typography.headlineMediumEmphasized.nums(),
                color = if (danger) tone.base else scheme.onSurface,
            )
        }
        AttendanceLine(course.percent, tone.base, Modifier.padding(vertical = Space.md))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                if (course.total == 0) "No classes yet" else "$attended/$held${if (course.isLab) " sessions" else ""}",
                Modifier.weight(1f),
                style = MaterialTheme.typography.bodyMedium.nums(),
                color = scheme.onSurfaceVariant,
            )
            if (danger) StatusChip("Debarred", tone)
            else advice?.let { StatusChip(it, tone) }
        }
        debarLine?.let {
            Text(
                it,
                Modifier.padding(top = Space.sm),
                style = MaterialTheme.typography.labelLarge.nums(),
                color = tone.onSubtle,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis,
            )
        }
        plan?.let { OutlookLine(course, it, showSkipAdvice, tone.onSubtle, Modifier.padding(top = Space.md)) }
    }
}
