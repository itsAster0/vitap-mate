package vitapmate.features.grades

import androidx.compose.foundation.layout.Column
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import vitapmate.core.GradeHistory
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.nums

/** VTOP's CGPA front and centre, credits earned and registered beside it, the grade mix and the GPA by session. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun CgpaCard(history: GradeHistory, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val label = MaterialTheme.typography.labelSmall
    val trend = history.sessions.mapNotNull { s -> s.gpa?.let { s.short to it } }
    SectionCard(modifier.fillMaxWidth().testTag("grades.cgpa")) {
        val who = listOf(history.name, history.regNo).filter { it.isNotBlank() }.joinToString(" · ")
        if (who.isNotEmpty()) {
            Text(who, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Spacer(Modifier.height(Space.md))
        }
        Row(verticalAlignment = Alignment.Bottom) {
            Column(Modifier.weight(1f)) {
                Text("CGPA", style = label, fontWeight = FontWeight.SemiBold, color = scheme.onSurfaceVariant)
                val big = MaterialTheme.typography.displayLargeEmphasized.nums()
                val cgpa = history.cgpa.toDoubleOrNull()
                if (cgpa == null) Text(history.cgpa.ifEmpty { "—" }, style = big)
                else CountUp(cgpa, big, Modifier.testTag("grades.cgpa.value"))
            }
            Stat("CREDITS EARNED", history.creditsEarned)
            Spacer(Modifier.width(Space.lg))
            Stat("REGISTERED", history.creditsRegistered)
        }
        if (history.mix.isNotEmpty()) {
            Spacer(Modifier.height(Space.lg))
            GradeMix(history.mix)
        }
        if (trend.size >= 2) {
            Spacer(Modifier.height(Space.lg))
            Text("GPA BY SEMESTER", style = label, fontWeight = FontWeight.SemiBold, color = scheme.onSurfaceVariant)
            Spacer(Modifier.height(Space.sm))
            GpaTrend(trend)
        }
    }
}

@Composable
private fun Stat(label: String, value: String) {
    Column(horizontalAlignment = Alignment.End) {
        Text(label, style = MaterialTheme.typography.labelSmall, fontWeight = FontWeight.SemiBold, color = MaterialTheme.colorScheme.onSurfaceVariant)
        Text(value.ifEmpty { "–" }, style = MaterialTheme.typography.titleLargeEmphasized.nums())
    }
}
