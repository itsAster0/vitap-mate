package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme
import vitapmate.ui.nums

/** Whether a [GroupItem] sits in a [ListGroup] (a segment) or alone (no container). */
internal val LocalInGroup = staticCompositionLocalOf { false }

/**
 * A titled section of segmented rows (More, Settings): the title in the primary colour, then
 * [GroupItem]s and [GroupSegment]s as one rounded group with small gaps, then an optional note.
 * [titleEnd] is a short note at the title's end (a count); [belowTitle] sits under the title (a meter).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ListGroup(title: String?, modifier: Modifier = Modifier, footer: String? = null, titleEnd: String? = null, belowTitle: (@Composable () -> Unit)? = null, content: @Composable ColumnScope.() -> Unit) {
    Column(modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp)) {
        title?.let {
            Row(Modifier.padding(start = 16.dp, end = 16.dp, bottom = 8.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(
                    it,
                    Modifier.weight(1f).semantics { heading() },
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.primary,
                )
                titleEnd?.let { end -> Text(end, style = MaterialTheme.typography.labelMedium.nums(), color = MaterialTheme.colorScheme.onSurfaceVariant) }
            }
        }
        belowTitle?.let { Box(Modifier.padding(start = 16.dp, end = 16.dp, bottom = 10.dp)) { it() } }
        // The outer clip rounds the group's ends; each row keeps the small inner corners.
        Column(Modifier.clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
            CompositionLocalProvider(LocalInGroup provides true) { content() }
        }
        footer?.let {
            Text(
                it,
                Modifier.padding(start = 16.dp, top = 8.dp, end = 16.dp),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

/** A segment of a [ListGroup] holding anything other than a [GroupItem] (a picker, a row of swatches). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun GroupSegment(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) = Column(
    modifier.fillMaxWidth().clip(ListItemDefaults.segmentedShapes(1, 3).shape).background(AppTheme.colors.card).padding(horizontal = 16.dp, vertical = 12.dp),
    content = content,
)
