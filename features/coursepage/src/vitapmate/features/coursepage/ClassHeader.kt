package vitapmate.features.coursepage

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ArrowLeftRight
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.Lucide
import vitapmate.core.CourseView
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonSize
import vitapmate.ui.ButtonVariant
import vitapmate.ui.ProgressBar
import vitapmate.ui.Radii
import vitapmate.ui.Space
import vitapmate.ui.ToneBadge
import vitapmate.ui.heroCookie
import vitapmate.ui.nums

/**
 * The shown section as a hero: "code · part", the course title, the faculty's initials with
 * faculty and school, the slot with YOUR / OTHER SECTION, how many lectures are covered (dated
 * before [today], as in "Covered"), and with several sections a button to see them all ([onSwitch]).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ClassHeader(view: CourseView, today: Int, onSwitch: (() -> Unit)?, modifier: Modifier = Modifier) {
    val section = view.sections.firstOrNull { it.classId == view.selected } ?: return
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val lectures = view.plan?.lectures.orEmpty()
    Column(
        modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge).background(tones.accentTone.subtle).heroCookie()
            .testTag("coursepage.header").padding(18.dp),
    ) {
        Text(
            listOf(view.course.code, view.course.label).filter { it.isNotBlank() }.joinToString(" · "),
            style = MaterialTheme.typography.labelMedium,
            color = tones.accentTone.onSubtle,
        )
        Text(view.course.title, style = MaterialTheme.typography.headlineSmallEmphasized, color = scheme.onSurface)
        Spacer(Modifier.height(Space.md))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(42.dp).background(AppTheme.colors.card, CircleShape), Alignment.Center) {
                Text(initials(section.faculty), style = MaterialTheme.typography.titleSmall, color = scheme.onSurface)
            }
            Spacer(Modifier.width(Space.md))
            Column(Modifier.weight(1f)) {
                Text(section.faculty, style = MaterialTheme.typography.titleSmall, color = scheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
                if (section.school.isNotBlank()) {
                    Text(section.school, style = MaterialTheme.typography.bodyMedium, color = tones.accentTone.onSubtle, maxLines = 1, overflow = TextOverflow.Ellipsis)
                }
            }
        }
        Spacer(Modifier.height(Space.md))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Row(
                Modifier.weight(1f, fill = false).background(AppTheme.colors.card, RoundedCornerShape(Radii.pill)).padding(horizontal = 10.dp, vertical = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(Lucide.Clock, null, Modifier.size(14.dp), tint = scheme.onSurfaceVariant)
                Spacer(Modifier.width(6.dp))
                Text("Slot ${section.slot}", style = MaterialTheme.typography.labelLarge, color = scheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
            Spacer(Modifier.width(Space.sm))
            if (section.mine) ToneBadge("YOUR SECTION", tone = tones.accentTone, solid = true) else ToneBadge("OTHER SECTION")
        }
        if (lectures.isNotEmpty()) {
            val covered = lectures.count { (it.day ?: Int.MIN_VALUE) < today }
            ProgressBar(covered.toFloat() / lectures.size, tones.accent, Modifier.padding(top = Space.md))
            Text(
                "$covered of ${lectures.size} lectures covered",
                Modifier.padding(top = 6.dp),
                style = MaterialTheme.typography.labelMedium.nums(),
                color = tones.accentTone.onSubtle,
            )
        }
        onSwitch?.let {
            Button(it, Modifier.padding(top = Space.md).testTag("coursepage.sections.open"), ButtonVariant.Outline, ButtonSize.Sm, prefix = Lucide.ArrowLeftRight) {
                Text("See all ${view.sections.size} sections")
            }
        }
    }
}

/** "Dr. S. Lakshmi" → "SL": the first letters of the first and last names, titles skipped. */
internal fun initials(name: String): String {
    val words = name.split(' ', '.').filter { it.isNotBlank() && it.lowercase() !in TITLES }
    return listOfNotNull(words.firstOrNull(), words.drop(1).lastOrNull()).joinToString("") { it.take(1).uppercase() }.ifEmpty { "•" }
}

private val TITLES = setOf("dr", "prof", "mr", "mrs", "ms")
