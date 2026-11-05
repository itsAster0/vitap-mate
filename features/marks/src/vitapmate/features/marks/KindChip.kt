package vitapmate.features.marks

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.Lucide
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip

/** "Theory" in the accent tone or "Lab" in the lab tone. */
@Composable
internal fun KindChip(lab: Boolean, modifier: Modifier = Modifier) {
    val tones = AppTheme.tones
    if (lab) StatusChip("Lab", tones.lab, modifier, Lucide.FlaskConical)
    else StatusChip("Theory", tones.accentTone, modifier, Lucide.BookOpen)
}
