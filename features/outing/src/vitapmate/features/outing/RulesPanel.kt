package vitapmate.features.outing

import androidx.compose.foundation.layout.Column
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Plus
import vitapmate.core.OutingFact
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.Space
import vitapmate.ui.nums

/**
 * The form's rules, two facts a row (a small label over a short value), and New request. While
 * VTOP takes no applications: that, with VTOP's [notice], and the button off.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun RulesPanel(open: Boolean, notice: String, facts: List<OutingFact>, onApply: () -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    Card(
        modifier.fillMaxWidth().testTag("outing.rules"),
        shape = MaterialTheme.shapes.large,
        colors = CardDefaults.cardColors(containerColor = AppTheme.colors.card),
    ) {
        Column(Modifier.padding(Space.lg)) {
            if (!open) {
                Text("Applications are closed", style = MaterialTheme.typography.titleSmallEmphasized)
                if (notice.isNotBlank()) {
                    Spacer(Modifier.height(2.dp))
                    Text(notice, Modifier.testTag("outing.notice"), style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
                }
                Spacer(Modifier.height(Space.md))
            }
            facts.chunked(2).forEachIndexed { i, row ->
                if (i > 0) Spacer(Modifier.height(Space.md))
                Row {
                    row.forEachIndexed { j, fact ->
                        if (j > 0) Spacer(Modifier.width(Space.md))
                        Column(Modifier.weight(1f)) {
                            Text(fact.label, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
                            Text(fact.value, style = MaterialTheme.typography.titleSmall.nums())
                        }
                    }
                    if (row.size == 1) {
                        Spacer(Modifier.width(Space.md))
                        Spacer(Modifier.weight(1f))
                    }
                }
            }
            Spacer(Modifier.height(Space.lg))
            Button(if (open) onApply else null, Modifier.fillMaxWidth().testTag("outing.apply"), prefix = Lucide.Plus) {
                Text(if (open) "New request" else "Applications closed")
            }
        }
    }
}
