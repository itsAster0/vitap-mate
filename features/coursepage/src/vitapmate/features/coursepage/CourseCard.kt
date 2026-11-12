package vitapmate.features.coursepage

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.GraduationCap
import com.composables.icons.lucide.Lucide
import vitapmate.core.CourseGroup
import vitapmate.core.CoursePart
import vitapmate.ui.AppTheme
import vitapmate.ui.SectionCard
import vitapmate.ui.Space
import vitapmate.ui.tappable

/**
 * A shaped mark, the course code over its title, and one joined row of tonal buttons, one per
 * part (lab in the lab tone): small inner corners, large outer ones.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun CourseCard(group: CourseGroup, onOpen: (CoursePart) -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    SectionCard(modifier.fillMaxWidth()) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(44.dp).background(scheme.secondaryContainer, MaterialShapes.Cookie7Sided.toShape()), Alignment.Center) {
                Icon(Lucide.GraduationCap, null, Modifier.size(20.dp), tint = scheme.onSecondaryContainer)
            }
            Spacer(Modifier.width(Space.md))
            Column(Modifier.weight(1f)) {
                Text(group.code, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
                Text(group.title, style = MaterialTheme.typography.titleMediumEmphasized, maxLines = 2, overflow = TextOverflow.Ellipsis)
            }
        }
        Spacer(Modifier.height(Space.md))
        Row(Modifier.fillMaxWidth().clip(MaterialTheme.shapes.large), horizontalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
            group.parts.forEach { part ->
                val tone = if (part.lab) AppTheme.tones.lab else AppTheme.tones.accentTone
                Row(
                    Modifier.weight(1f)
                        .height(44.dp)
                        .background(tone.subtle, ListItemDefaults.segmentedShapes(1, 3).shape)
                        .tappable({ onOpen(part) }, part.label, Role.Button, haptic = true, pressedScale = 0.97f)
                        .testTag("coursepage.part.${part.id}")
                        .padding(horizontal = Space.sm),
                    horizontalArrangement = Arrangement.Center,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Icon(if (part.lab) Lucide.FlaskConical else Lucide.BookOpen, null, Modifier.size(16.dp), tint = tone.onSubtle)
                    Spacer(Modifier.width(Space.sm))
                    Text(part.label, style = MaterialTheme.typography.labelLarge, color = tone.onSubtle, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Spacer(Modifier.width(Space.xs))
                    Icon(Lucide.ChevronRight, null, Modifier.size(15.dp), tint = tone.onSubtle)
                }
            }
        }
    }
}
