package vitapmate.features.grades

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** A grade letter on its tone's fill, in a rounded square. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun GradeBadge(grade: String, modifier: Modifier = Modifier, size: Dp = 40.dp) {
    val tone = gradeTone(grade)
    Box(modifier.size(size).background(tone.subtle, MaterialTheme.shapes.medium), Alignment.Center) {
        Text(grade.trim().uppercase().ifEmpty { "–" }, style = MaterialTheme.typography.titleMediumEmphasized, color = tone.onSubtle)
    }
}
