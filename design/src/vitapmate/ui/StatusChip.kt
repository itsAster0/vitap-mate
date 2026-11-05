package vitapmate.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp

/** A small status label on a tone's fill (e.g. "Safe", "Lab FAT"); [solid] uses the tone's base colour. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun StatusChip(label: String, tone: Tone, modifier: Modifier = Modifier, icon: ImageVector? = null, solid: Boolean = false) {
    val fg = if (solid) MaterialTheme.colorScheme.surface else tone.onSubtle
    Row(
        modifier.background(if (solid) tone.base else tone.subtle, MaterialTheme.shapes.small).padding(horizontal = Space.sm, vertical = 3.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        icon?.let {
            Icon(it, null, Modifier.size(14.dp), tint = if (solid) fg else tone.base)
            Spacer(Modifier.width(Space.xs))
        }
        Text(label, style = MaterialTheme.typography.labelMediumEmphasized.nums(), color = fg, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}
