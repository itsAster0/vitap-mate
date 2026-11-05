package vitapmate.features.timetable

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import vitapmate.ui.nums

/** A section's title with an optional count or range on the right. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SectionTitle(title: String, trailing: String? = null) {
    Row(Modifier.fillMaxWidth().padding(start = 4.dp, top = 24.dp, end = 4.dp, bottom = 10.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(title, Modifier.weight(1f).semantics { heading() }, style = MaterialTheme.typography.titleMediumEmphasized)
        if (trailing != null) Text(trailing, style = MaterialTheme.typography.labelLarge.nums(), color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
