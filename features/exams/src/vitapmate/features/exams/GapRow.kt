package vitapmate.features.exams

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.BookOpenCheck
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Space

/** "2 days to prepare" on a hairline, between two exam days. */
@Composable
internal fun GapRow(days: Int, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    Row(modifier.fillMaxWidth().padding(top = Space.xs), verticalAlignment = Alignment.CenterVertically) {
        HorizontalDivider(Modifier.weight(1f), color = scheme.outlineVariant)
        Icon(Lucide.BookOpenCheck, null, Modifier.padding(start = Space.sm).size(12.dp), tint = scheme.onSurfaceVariant)
        Spacer(Modifier.width(Space.xs))
        Text(
            "$days ${if (days == 1) "day" else "days"} to prepare",
            Modifier.padding(end = Space.sm),
            style = MaterialTheme.typography.labelMedium,
            color = scheme.onSurfaceVariant,
        )
        HorizontalDivider(Modifier.weight(1f), color = scheme.outlineVariant)
    }
}
