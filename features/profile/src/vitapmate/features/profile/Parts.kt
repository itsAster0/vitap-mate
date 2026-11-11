package vitapmate.features.profile

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.Tone

/** A section's title in the primary colour, like Settings'. */
@Composable
internal fun SectionTitle(text: String, modifier: Modifier = Modifier) = Text(
    text,
    modifier.padding(start = Space.lg, top = Space.md, bottom = Space.sm).semantics { heading() },
    style = MaterialTheme.typography.titleSmall,
    color = MaterialTheme.colorScheme.primary,
)

/** Segments stacked with small gaps, the group's ends rounded (Settings' list groups). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun Segmented(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) = Column(
    modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge),
    verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap),
    content = content,
)

/** A middle segment's small corners: the [Segmented] clip rounds the ends. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal val SegmentShape: Shape @Composable get() = ListItemDefaults.segmentedShapes(1, 3).shape

/** One segment of a [Segmented] group, on [color] (the card colour by default). */
@Composable
internal fun Segment(modifier: Modifier = Modifier, color: Color = AppTheme.colors.card, content: @Composable ColumnScope.() -> Unit) = Column(
    modifier.fillMaxWidth().clip(SegmentShape).background(color).padding(horizontal = Space.lg, vertical = Space.md),
    content = content,
)

/** [icon] in [tone]'s fill, in a circle unless [shape] says otherwise. */
@Composable
internal fun IconCircle(icon: ImageVector, tone: Tone, size: Dp = 42.dp, shape: Shape = CircleShape) =
    Box(Modifier.size(size).background(tone.subtle, shape), Alignment.Center) {
        Icon(icon, null, Modifier.size(20.dp), tint = tone.base)
    }
