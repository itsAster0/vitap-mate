package vitapmate.ui

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ProvideTextStyle
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp

/** Section title with an optional trailing slot (count, action). */
@Composable
fun SectionHeader(title: String, modifier: Modifier = Modifier, trailing: (@Composable () -> Unit)? = null) {
    val colors = AppTheme.colors
    val type = AppTheme.type
    Row(modifier.padding(start = 2.dp, top = Space.lg, end = 2.dp, bottom = Space.sm + 2.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(title, Modifier.weight(1f), style = type.lg.copy(fontWeight = FontWeight.W700, color = colors.foreground))
        trailing?.let { ProvideTextStyle(type.sm.copy(color = colors.mutedForeground), it) }
    }
}
