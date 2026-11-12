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
import androidx.compose.material3.Card
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Lucide
import vitapmate.core.CourseSection
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.ToneBadge
import vitapmate.ui.tappable

/**
 * One section: the faculty's initials (in a shaped accent mark when it is the student's, with
 * YOURS), faculty, slot and school; the [selected] one ticked on the accent fill.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun SectionRow(section: CourseSection, selected: Boolean, onPress: () -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    val accent = AppTheme.tones.accentTone
    Card(
        modifier.fillMaxWidth()
            .tappable(onPress, section.faculty, Role.Button, haptic = true, pressedScale = 0.98f)
            .testTag("coursepage.section.${section.classId}"),
        shape = MaterialTheme.shapes.large,
        colors = CardDefaults.cardColors(containerColor = if (selected) accent.subtle else AppTheme.colors.card),
    ) {
        Row(Modifier.padding(horizontal = Space.lg, vertical = Space.md), verticalAlignment = Alignment.CenterVertically) {
            Box(
                Modifier.size(42.dp).background(
                    if (section.mine) AppTheme.tones.accent else scheme.secondaryContainer,
                    if (section.mine) MaterialShapes.Cookie9Sided.toShape() else CircleShape,
                ),
                Alignment.Center,
            ) {
                Text(
                    initials(section.faculty),
                    style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.W700),
                    color = if (section.mine) AppTheme.tones.onAccent else scheme.onSecondaryContainer,
                )
            }
            Spacer(Modifier.width(Space.md))
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        section.faculty,
                        Modifier.weight(1f, fill = false),
                        style = MaterialTheme.typography.titleSmall.copy(fontWeight = FontWeight.W700),
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    if (section.mine) {
                        Spacer(Modifier.width(Space.sm))
                        ToneBadge("YOURS", tone = accent)
                    }
                }
                Spacer(Modifier.height(2.dp))
                Text(
                    listOf(section.slot, section.school).filter { it.isNotBlank() }.joinToString(" · "),
                    style = MaterialTheme.typography.labelMedium,
                    color = scheme.onSurfaceVariant,
                )
            }
            Icon(
                if (selected) Lucide.Check else Lucide.ChevronRight,
                null,
                Modifier.size(16.dp),
                tint = if (selected) accent.base else scheme.onSurfaceVariant,
            )
        }
    }
}
