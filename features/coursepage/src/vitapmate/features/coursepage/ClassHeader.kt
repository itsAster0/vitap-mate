package vitapmate.features.coursepage

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ArrowLeftRight
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.User
import vitapmate.core.CourseView
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonSize
import vitapmate.ui.ButtonVariant
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.ToneBadge

/**
 * The shown section: course title, faculty and school, slot with YOUR / OTHER SECTION, and with
 * several sections a button to see them all ([onSwitch]).
 */
@Composable
internal fun ClassHeader(view: CourseView, onSwitch: (() -> Unit)?, modifier: Modifier = Modifier) {
    val section = view.sections.firstOrNull { it.classId == view.selected } ?: return
    SectionCard(modifier.fillMaxWidth().testTag("coursepage.header")) {
        CourseTitle(view.course)
        Spacer(Modifier.height(Space.sm))
        Fact(Lucide.User, listOf(section.faculty, section.school).filter { it.isNotBlank() }.joinToString(" · "))
        Spacer(Modifier.height(Space.xs))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Fact(Lucide.Clock, "Slot ${section.slot}", Modifier.weight(1f, fill = false))
            Spacer(Modifier.width(Space.sm))
            if (section.mine) ToneBadge("YOUR SECTION", tone = AppTheme.tones.accentTone) else ToneBadge("OTHER SECTION")
        }
        onSwitch?.let {
            Button(it, Modifier.padding(top = Space.md).testTag("coursepage.sections.open"), ButtonVariant.Outline, ButtonSize.Sm, prefix = Lucide.ArrowLeftRight) {
                Text("See all ${view.sections.size} sections")
            }
        }
    }
}

@Composable
private fun Fact(icon: ImageVector, text: String, modifier: Modifier = Modifier) {
    val muted = MaterialTheme.colorScheme.onSurfaceVariant
    Row(modifier, verticalAlignment = Alignment.CenterVertically) {
        Icon(icon, null, Modifier.size(14.dp), tint = muted)
        Spacer(Modifier.width(6.dp))
        Text(text, style = MaterialTheme.typography.bodyMedium, color = muted, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}
