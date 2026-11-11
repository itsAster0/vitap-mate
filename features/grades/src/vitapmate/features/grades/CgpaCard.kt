package vitapmate.features.grades

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import vitapmate.core.GradeHistory
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.nums

/**
 * VTOP's CGPA front and centre, credits earned and registered as two tiles under it and the grade
 * mix; the GPA by session in a block attached below.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun CgpaCard(history: GradeHistory, modifier: Modifier = Modifier) {
    val muted = AppTheme.tones.accentTone.onSubtle
    val label = MaterialTheme.typography.labelMedium
    val trend = history.sessions.mapNotNull { s -> s.gpa?.let { s.short to it } }
    val blocks = if (trend.size >= 2) 2 else 1
    Column(modifier.fillMaxWidth().testTag("grades.cgpa"), verticalArrangement = Arrangement.spacedBy(3.dp)) {
        AccentBlock(Modifier.fillMaxWidth(), attached(0, blocks)) {
            val who = listOf(history.name, history.regNo).filter { it.isNotBlank() }.joinToString(" · ")
            if (who.isNotEmpty()) {
                Text(who, style = label, fontWeight = FontWeight.SemiBold, color = muted, maxLines = 1, overflow = TextOverflow.Ellipsis)
                Spacer(Modifier.height(Space.md))
            }
            Text("CGPA", style = label, fontWeight = FontWeight.SemiBold, color = muted)
            val big = MaterialTheme.typography.displayLargeEmphasized.nums(fontSize = 68.sp, lineHeight = 72.sp)
            val cgpa = history.cgpa.toDoubleOrNull()
            if (cgpa == null) Text(history.cgpa.ifEmpty { "—" }, style = big)
            else CountUp(cgpa, big, Modifier.testTag("grades.cgpa.value"))
            Spacer(Modifier.height(Space.md))
            Row(horizontalArrangement = Arrangement.spacedBy(3.dp)) {
                Stat("Credits earned", history.creditsEarned, Modifier.weight(1f))
                Stat("Registered", history.creditsRegistered, Modifier.weight(1f))
            }
            if (history.mix.isNotEmpty()) {
                Spacer(Modifier.height(Space.lg))
                GradeMix(history.mix)
            }
        }
        if (blocks == 2) {
            AccentBlock(Modifier.fillMaxWidth(), attached(1, blocks)) {
                Text("GPA by semester", style = label, fontWeight = FontWeight.SemiBold, color = muted)
                Spacer(Modifier.height(Space.sm))
                GpaTrend(trend)
            }
        }
    }
}

/** A credits figure over its label, on a tile a shade off the accent block. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Stat(label: String, value: String, modifier: Modifier) {
    Column(modifier.background(MaterialTheme.colorScheme.surface.copy(alpha = 0.35f), RoundedCornerShape(14.dp)).padding(horizontal = Space.md, vertical = 10.dp)) {
        Text(value.ifEmpty { "–" }, style = MaterialTheme.typography.titleLargeEmphasized.nums())
        Text(label, style = MaterialTheme.typography.labelMedium, fontWeight = FontWeight.SemiBold, color = AppTheme.tones.accentTone.onSubtle)
    }
}
