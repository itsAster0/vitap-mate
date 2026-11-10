package vitapmate.features.timetable

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import vitapmate.ui.nums

/** The room, large, in an expressive cookie shape (it's what you look for on the way), its block under it. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun RoomTile(room: String, block: String, fill: Color, onFill: Color, size: Dp, blockColor: Color = MaterialTheme.colorScheme.onSurfaceVariant, modifier: Modifier = Modifier) {
    Column(
        modifier.width(size + 4.dp).clearAndSetSemantics { contentDescription = "Room $room, $block" },
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Box(Modifier.size(size).background(fill, MaterialShapes.Cookie9Sided.toShape()).padding(14.dp), Alignment.Center) {
            Text(room.ifBlank { "–" }, Modifier.fitDown(), maxLines = 1, softWrap = false, style = MaterialTheme.typography.headlineSmallEmphasized.nums(), color = onFill)
        }
        if (block.isNotBlank()) {
            Spacer(Modifier.height(4.dp))
            Text(block, maxLines = 1, overflow = TextOverflow.Ellipsis, textAlign = TextAlign.Center, style = MaterialTheme.typography.labelMedium, color = blockColor)
        }
    }
}
