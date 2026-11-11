package vitapmate.features.grades

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** A grade letter on its tone's fill, in a rounded square (an S in a cookie). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun GradeBadge(grade: String, modifier: Modifier = Modifier, size: Dp = 44.dp) {
    val tone = gradeTone(grade)
    val letter = grade.trim().uppercase()
    val shape = if (letter == "S") MaterialShapes.Cookie9Sided.toShape() else RoundedCornerShape(14.dp)
    Box(modifier.size(size).background(tone.subtle, shape), Alignment.Center) {
        Text(letter.ifEmpty { "–" }, style = MaterialTheme.typography.titleMediumEmphasized, color = tone.onSubtle)
    }
}
