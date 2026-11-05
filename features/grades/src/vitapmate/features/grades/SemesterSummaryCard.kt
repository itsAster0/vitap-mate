package vitapmate.features.grades

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import vitapmate.core.GradeCourse
import vitapmate.core.SemesterSummary
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.nums
import vitapmate.ui.tappable

/**
 * The semester's GPA (credit-weighted, credits from the grade history), a bar with a segment per
 * course as wide as its credits in its grade's tone (tap one to jump to its card), and the grade mix.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun SemesterSummaryCard(courses: List<GradeCourse>, summary: SemesterSummary, onCourse: (Int) -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val label = MaterialTheme.typography.labelSmall
    SectionCard(modifier.fillMaxWidth().testTag("grades.summary")) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text("SEMESTER GPA", Modifier.weight(1f), style = label, fontWeight = FontWeight.SemiBold, color = scheme.onSurfaceVariant)
            if (summary.gpa != null) Text("${summary.credits.creditsLabel()} credits", style = label.nums(), fontWeight = FontWeight.SemiBold, color = scheme.onSurfaceVariant)
        }
        Row(verticalAlignment = Alignment.Bottom) {
            val big = MaterialTheme.typography.displayMediumEmphasized.nums()
            val gpa = summary.gpa
            if (gpa == null) Text("—", Modifier.weight(1f).testTag("grades.gpa"), style = big, color = scheme.onSurfaceVariant)
            else CountUp(gpa, big, Modifier.weight(1f).testTag("grades.gpa"))
            Text("${courses.size} courses", Modifier.padding(bottom = 6.dp), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        }
        Spacer(Modifier.height(Space.md))
        var shown by remember { mutableStateOf(false) }
        LaunchedEffect(Unit) { shown = true }
        val fade by animateFloatAsState(if (shown) 1f else 0f, MaterialTheme.motionScheme.slowEffectsSpec())
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(3.dp)) {
            courses.forEachIndexed { i, course ->
                // Non-credit courses still get a sliver.
                val flex = maxOf(5, Math.round((summary.courseCredits.getOrNull(i) ?: 0.0) * 10).toInt())
                Box(
                    Modifier.weight(flex.toFloat())
                        .tappable({ onCourse(i) }, "${course.title}, grade ${course.grade}", Role.Button, pressedScale = 0.9f)
                        .padding(vertical = 6.dp)
                        .testTag("grades.summary.course.$i"),
                ) {
                    Box(Modifier.fillMaxWidth().height(10.dp).alpha(fade).background(gradeTone(course.grade).base, RoundedCornerShape(3.dp)))
                }
            }
        }
        if (summary.counts.isNotEmpty()) {
            Spacer(Modifier.height(Space.sm))
            GradeLegend(summary.counts)
        }
    }
}
