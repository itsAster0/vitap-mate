package vitapmate.features.grades

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
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
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.nums
import vitapmate.ui.tappable

/**
 * The semester's GPA (credit-weighted, credits from the grade history), a bar with a segment per
 * course as wide as its credits in its grade's tone, lettered when it fits (tap one to jump to its
 * card), and the grade mix.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun SemesterSummaryCard(courses: List<GradeCourse>, summary: SemesterSummary, onCourse: (Int) -> Unit, modifier: Modifier = Modifier) {
    val muted = AppTheme.tones.accentTone.onSubtle
    val side = MaterialTheme.typography.labelLarge.nums()
    AccentBlock(modifier.fillMaxWidth().testTag("grades.summary")) {
        Row(verticalAlignment = Alignment.Bottom) {
            Column(Modifier.weight(1f)) {
                Text("Semester GPA", style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.SemiBold, color = muted)
                val big = MaterialTheme.typography.displayLargeEmphasized.nums()
                val gpa = summary.gpa
                if (gpa == null) Text("—", Modifier.testTag("grades.gpa"), style = big, color = muted)
                else CountUp(gpa, big, Modifier.testTag("grades.gpa"))
            }
            Column(Modifier.padding(bottom = 8.dp), horizontalAlignment = Alignment.End) {
                if (summary.gpa != null) Text("${summary.credits.creditsLabel()} credits", style = side, fontWeight = FontWeight.SemiBold, color = muted)
                Text("${courses.size} courses", style = side, fontWeight = FontWeight.SemiBold, color = muted)
            }
        }
        Spacer(Modifier.height(Space.sm))
        var shown by remember { mutableStateOf(false) }
        LaunchedEffect(Unit) { shown = true }
        val fade by animateFloatAsState(if (shown) 1f else 0f, MaterialTheme.motionScheme.slowEffectsSpec())
        val letter = MaterialTheme.typography.labelLargeEmphasized
        val onFill = AppTheme.colors.background
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
                    BoxWithConstraints(
                        Modifier.fillMaxWidth().height(30.dp).alpha(fade).background(gradeTone(course.grade).base, connected(i, courses.size, 15.dp, 9.dp)),
                        contentAlignment = Alignment.Center,
                    ) {
                        val grade = course.grade.trim().uppercase()
                        if (grade.isNotEmpty() && maxWidth >= 16.dp) Text(grade, style = letter, color = onFill, maxLines = 1)
                    }
                }
            }
        }
        if (summary.counts.isNotEmpty()) {
            Spacer(Modifier.height(Space.xs))
            GradeLegend(summary.counts)
        }
    }
}
