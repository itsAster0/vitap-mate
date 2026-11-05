package vitapmate.features.timetable

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.TriangleAlert
import vitapmate.ui.AppTheme
import vitapmate.ui.StatusChip

/**
 * Lab FAT, Last class, Clash (overlaps another class) and [note] ("No lab") when they apply. Theory
 * and lab need no tag (the code and the rail's colour tell them apart).
 */
@Composable
fun ClassTags(modifier: Modifier = Modifier, labFat: Boolean = false, lastClass: Boolean = false, clash: Boolean = false, note: String? = null) {
    val tones = AppTheme.tones
    FlowRow(modifier, horizontalArrangement = Arrangement.spacedBy(6.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        if (labFat) StatusChip("Lab FAT", tones.warning, icon = Lucide.FlaskConical)
        if (lastClass) StatusChip("Last class", tones.accentTone)
        if (clash) StatusChip("Clash", tones.danger, icon = Lucide.TriangleAlert)
        if (note != null) StatusChip(note, tones.warning)
    }
}
