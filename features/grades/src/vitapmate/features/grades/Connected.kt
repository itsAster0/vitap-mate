package vitapmate.features.grades

import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import vitapmate.ui.AppTheme
import vitapmate.ui.Radii
import vitapmate.ui.Space

/** Item [i] of [count] joined in a row (or a column with [vertical]): [outer] corners at the ends, [inner] between. */
internal fun connected(i: Int, count: Int, outer: Dp, inner: Dp, vertical: Boolean = false): RoundedCornerShape {
    val start = if (i == 0) outer else inner
    val end = if (i == count - 1) outer else inner
    return if (vertical) RoundedCornerShape(start, start, end, end) else RoundedCornerShape(start, end, end, start)
}

/** Card [i] of [count] in a segmented list: 28 dp at the run's ends, 6 dp between (with 3 dp gaps). */
internal fun segment(i: Int, count: Int): Shape = connected(i, count, 28.dp, 6.dp, vertical = true)

/** Block [i] of [count] attached summary blocks, the group rounded like extraLarge. */
internal fun attached(i: Int, count: Int): Shape = connected(i, count, Radii.xl, 6.dp, vertical = true)

/** A summary block on the accent's subtle fill, numbers in onSurface (labels set onSubtle themselves). */
@Composable
internal fun AccentBlock(modifier: Modifier = Modifier, shape: Shape = MaterialTheme.shapes.extraLarge, content: @Composable ColumnScope.() -> Unit) {
    val colors = CardDefaults.cardColors(containerColor = AppTheme.tones.accentTone.subtle, contentColor = MaterialTheme.colorScheme.onSurface)
    Card(modifier, shape, colors) { Column(Modifier.padding(Space.lg), content = content) }
}

/** A 34 dp tonal circle for a stepper's − or +; disabled when [onPress] is null. */
@Composable
internal fun StepButton(icon: ImageVector, onPress: (() -> Unit)?, label: String, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    FilledTonalIconButton(
        onPress ?: {},
        modifier.size(34.dp),
        enabled = onPress != null,
        shape = CircleShape,
        colors = IconButtonDefaults.filledTonalIconButtonColors(containerColor = scheme.surfaceContainerHigh, contentColor = scheme.onSurface),
    ) { Icon(icon, label, Modifier.size(16.dp)) }
}
