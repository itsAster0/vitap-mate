package vitapmate.features.grades

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
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
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.RefreshCw
import vitapmate.core.GradeCourse
import vitapmate.core.GradeCutoff
import vitapmate.core.GradeDetails
import vitapmate.core.GradeMark
import vitapmate.ui.ProgressBar
import vitapmate.ui.Skeleton
import vitapmate.ui.Space
import vitapmate.ui.nums

/** A course's grade cut-offs (its own highlighted) and marks by section, with a refresh button. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun GradeDetailsPanel(course: GradeCourse, details: GradeDetails?, loading: Boolean, failed: Boolean, onRefresh: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Column(Modifier.fillMaxWidth().padding(top = Space.md).testTag("grades.course.${course.courseId}.details")) {
        HorizontalDivider(color = scheme.outlineVariant)
        if (details == null) {
            if (loading || !failed) {
                Column(Modifier.padding(top = Space.md), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                    Skeleton(height = 28.dp, radius = 8.dp)
                    Skeleton(height = 44.dp, radius = 8.dp)
                    Skeleton(height = 44.dp, radius = 8.dp)
                }
            } else {
                Row(Modifier.fillMaxWidth().padding(top = Space.md), verticalAlignment = Alignment.CenterVertically) {
                    Text("Couldn't load the breakdown", Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
                    TextButton(onRefresh, Modifier.testTag("grades.course.${course.courseId}.retry")) { Text("Try again") }
                }
            }
            return@Column
        }
        if (details.cutoffs.isNotEmpty()) {
            Label(listOf("Grade cut-offs", course.gradingType).filter { it.isNotBlank() }.joinToString(" · "), Modifier.padding(top = Space.md, bottom = Space.sm))
            FlowRow(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                details.cutoffs.forEach { Cutoff(it, achieved = it.grade == course.grade) }
            }
        }
        details.sections.forEach { section ->
            Row(Modifier.fillMaxWidth().padding(top = Space.lg, bottom = Space.xs), verticalAlignment = Alignment.CenterVertically) {
                Label(section.title, Modifier.weight(1f))
                if (section.total.isNotEmpty()) {
                    Text("${section.total} weighted", style = MaterialTheme.typography.labelMedium.nums(), fontWeight = FontWeight.SemiBold)
                }
            }
            section.marks.forEachIndexed { i, mark ->
                if (i > 0) HorizontalDivider(color = scheme.outlineVariant.copy(alpha = 0.6f))
                MarkRow(mark)
            }
        }
        Row(Modifier.fillMaxWidth().padding(top = Space.xs), horizontalArrangement = Arrangement.End, verticalAlignment = Alignment.CenterVertically) {
            TextButton(onRefresh, Modifier.testTag("grades.course.${course.courseId}.refresh"), enabled = !loading) {
                if (loading) LoadingIndicator(Modifier.size(18.dp)) else Icon(Lucide.RefreshCw, null, Modifier.size(16.dp))
                Spacer(Modifier.width(Space.sm))
                Text("Refresh")
            }
        }
    }
}

@Composable
private fun Label(text: String, modifier: Modifier = Modifier) {
    Text(text.uppercase(), modifier, style = MaterialTheme.typography.labelSmall, fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
}

@Composable
private fun Cutoff(cutoff: GradeCutoff, achieved: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val tone = gradeTone(cutoff.grade)
    Box(
        Modifier
            .background(if (achieved) tone.subtle else Color.Transparent, MaterialTheme.shapes.small)
            .border(BorderStroke(1.dp, if (achieved) tone.base else scheme.outlineVariant), MaterialTheme.shapes.small)
            .padding(horizontal = Space.sm, vertical = 4.dp)
            .testTag("grades.cutoff.${cutoff.grade}"),
    ) {
        Text(
            buildAnnotatedString {
                withStyle(SpanStyle(fontWeight = FontWeight.Bold, color = if (achieved) tone.onSubtle else scheme.onSurface)) { append("${cutoff.grade} ") }
                withStyle(SpanStyle(color = if (achieved) tone.onSubtle else scheme.onSurfaceVariant)) { append(cutoff.range) }
            },
            style = MaterialTheme.typography.labelMedium.nums(),
        )
    }
}

/** A mark: title, scored / max, a bar of the score in its band's tone, weighted of weightage. */
@Composable
private fun MarkRow(mark: GradeMark) {
    val scheme = MaterialTheme.colorScheme
    Column(Modifier.fillMaxWidth().padding(vertical = Space.sm)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(mark.title, Modifier.weight(1f), style = MaterialTheme.typography.titleSmall, maxLines = 2, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.width(Space.sm))
            Text(
                buildAnnotatedString {
                    append(mark.scored.ifEmpty { "–" })
                    withStyle(SpanStyle(color = scheme.onSurfaceVariant)) { append(" / ${mark.max}") }
                },
                style = MaterialTheme.typography.titleSmall.nums(),
            )
        }
        Spacer(Modifier.height(6.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            ProgressBar(((mark.percent ?: 0.0) / 100).toFloat(), mark.band.tone().base, Modifier.weight(1f), height = 4.dp)
            Spacer(Modifier.width(Space.md))
            Text("${mark.weighted.ifEmpty { "–" }} of ${mark.weightage}", style = MaterialTheme.typography.labelMedium.nums(), color = scheme.onSurfaceVariant)
        }
    }
}
