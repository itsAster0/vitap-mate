package vitapmate.features.outing

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
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
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarX
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Plus
import vitapmate.core.OutingFact
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.Space
import vitapmate.ui.heroCookie
import vitapmate.ui.nums

/**
 * The form's rules as a header card (the accent fill while open): the facts in joined tiles, two
 * a row (a small label over a short value), and a full-width New request. While VTOP takes no
 * applications: a neutral card saying so, with VTOP's [notice], and the button off.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun RulesPanel(open: Boolean, notice: String, facts: List<OutingFact>, onApply: () -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    Column(
        modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge)
            .background(if (open) AppTheme.tones.accentTone.subtle else scheme.surfaceContainerHigh)
            .then(if (open) Modifier.heroCookie() else Modifier)
            .testTag("outing.rules").padding(Space.lg),
    ) {
        if (!open) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Box(Modifier.size(44.dp).background(scheme.secondaryContainer, MaterialShapes.Cookie7Sided.toShape()), Alignment.Center) {
                    Icon(Lucide.CalendarX, null, Modifier.size(20.dp), tint = scheme.onSecondaryContainer)
                }
                Spacer(Modifier.width(Space.md))
                Column(Modifier.weight(1f)) {
                    Text("Applications are closed", style = MaterialTheme.typography.titleSmallEmphasized)
                    if (notice.isNotBlank()) {
                        Spacer(Modifier.height(2.dp))
                        Text(notice, Modifier.testTag("outing.notice"), style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
                    }
                }
            }
            Spacer(Modifier.height(Space.md))
        }
        if (facts.isNotEmpty()) {
            Column(Modifier.clip(MaterialTheme.shapes.large), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                facts.chunked(2).forEach { row ->
                    Row(Modifier.height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                        row.forEach { fact ->
                            Column(
                                Modifier.weight(1f).fillMaxHeight()
                                    .background(AppTheme.colors.card, ListItemDefaults.segmentedShapes(1, 3).shape)
                                    .padding(horizontal = Space.md, vertical = 10.dp),
                            ) {
                                Text(fact.label, style = MaterialTheme.typography.labelSmall, color = scheme.onSurfaceVariant)
                                Text(fact.value, style = MaterialTheme.typography.titleSmall.nums())
                            }
                        }
                        if (row.size == 1) Spacer(Modifier.weight(1f))
                    }
                }
            }
            Spacer(Modifier.height(Space.lg))
        }
        Button(if (open) onApply else null, Modifier.fillMaxWidth().testTag("outing.apply"), prefix = Lucide.Plus) {
            Text(if (open) "New request" else "Applications closed")
        }
    }
}
