package vitapmate.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Copy
import com.composables.icons.lucide.Lucide

/** The registration number as an accent pill with a copy mark; a tap or long press copies it. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun RegnoPill(regno: String, modifier: Modifier = Modifier) {
    val tones = AppTheme.tones
    val copy = rememberCopy()
    Row(
        modifier.clip(CircleShape).background(tones.accent).copyable(regno) { copy(regno) }.height(34.dp).padding(horizontal = 13.dp),
        horizontalArrangement = Arrangement.spacedBy(7.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(regno, style = MaterialTheme.typography.labelLargeEmphasized.nums(), color = tones.onAccent)
        Icon(Lucide.Copy, null, Modifier.size(14.dp), tint = tones.onAccent)
    }
}
