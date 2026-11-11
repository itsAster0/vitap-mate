package vitapmate.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.Lucide

/**
 * "Theory" in the accent tone or "Lab" in the lab tone ("Project" with [project], a lab kind);
 * [fill] replaces the tone's quiet fill (e.g. on a tinted card).
 */
@Composable
fun KindChip(lab: Boolean, modifier: Modifier = Modifier, fill: Color? = null, project: Boolean = false) {
    val tones = AppTheme.tones
    val tone = if (lab) tones.lab else tones.accentTone
    StatusChip(if (project) "Project" else if (lab) "Lab" else "Theory", fill?.let { tone.copy(subtle = it) } ?: tone, modifier, if (lab) Lucide.FlaskConical else Lucide.BookOpen)
}
