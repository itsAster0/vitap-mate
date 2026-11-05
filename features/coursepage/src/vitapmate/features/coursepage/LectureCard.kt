package vitapmate.features.coursepage

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Paperclip
import vitapmate.core.Lecture
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.ToneBadge
import vitapmate.ui.formatDay
import vitapmate.ui.nums

/**
 * One lecture: its date, "#3 · TUE" with TODAY / NEXT ([next], on the accent fill) or PLANNED,
 * the topic, and its material as chips ([onMaterial] gets the path; [busy] paths spin). Planned
 * lectures after the next are muted.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class, ExperimentalLayoutApi::class)
@Composable
internal fun LectureCard(
    lecture: Lecture,
    today: Int,
    planned: Boolean,
    next: Boolean,
    busy: Set<String>,
    onMaterial: (String) -> Unit,
    modifier: Modifier = Modifier,
) {
    val scheme = MaterialTheme.colorScheme
    val accent = AppTheme.tones.accentTone
    val faded = planned && !next
    val strong = if (faded) scheme.onSurfaceVariant else scheme.onSurface
    val day = lecture.day
    Card(
        modifier.fillMaxWidth(),
        shape = MaterialTheme.shapes.large,
        colors = CardDefaults.cardColors(containerColor = if (next) accent.subtle else AppTheme.colors.card),
    ) {
        Row(Modifier.padding(start = Space.md, top = Space.md, end = Space.lg, bottom = if (lecture.materials.isEmpty()) Space.md else Space.sm), verticalAlignment = Alignment.Top) {
            Column(Modifier.width(44.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Text(day?.let { formatDay(it, "d") } ?: "—", style = MaterialTheme.typography.titleLargeEmphasized.nums(), color = strong)
                Text(
                    (day?.let { formatDay(it, "MMM") } ?: lecture.weekday).uppercase(),
                    style = MaterialTheme.typography.labelSmall,
                    color = scheme.onSurfaceVariant,
                )
            }
            Spacer(Modifier.width(Space.sm))
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        listOf("#${lecture.serial}", lecture.weekday).filter { it.isNotBlank() }.joinToString(" · "),
                        style = MaterialTheme.typography.labelMedium.nums(),
                        color = scheme.onSurfaceVariant,
                    )
                    when {
                        next -> {
                            Spacer(Modifier.width(Space.sm))
                            ToneBadge(if (day == today) "TODAY" else "NEXT", tone = accent, solid = true)
                        }
                        planned -> {
                            Spacer(Modifier.width(Space.sm))
                            ToneBadge("PLANNED")
                        }
                    }
                }
                Spacer(Modifier.height(2.dp))
                Text(lecture.topic.ifBlank { "No topic given" }, style = MaterialTheme.typography.bodyLarge, color = strong)
                if (lecture.materials.isNotEmpty()) {
                    // Chips keep 48dp touch targets: 8dp above and below each is already gap.
                    FlowRow(horizontalArrangement = Arrangement.spacedBy(Space.xs + 2.dp)) {
                        lecture.materials.forEach { material ->
                            FileChip(
                                Lucide.Paperclip,
                                material.label,
                                material.path in busy,
                                { onMaterial(material.path) },
                                Modifier.testTag("coursepage.material.${lecture.serial}"),
                                tonal = true,
                                solid = next,
                            )
                        }
                    }
                }
            }
        }
    }
}
