package vitapmate.features.outing

import androidx.compose.animation.animateColorAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import vitapmate.ui.Space
import vitapmate.ui.nums

/** A selectable pill (a day, a slot, a place); [caption] above the label (e.g. SUN). */
@Composable
internal fun ChoicePill(label: String, selected: Boolean, onPress: () -> Unit, modifier: Modifier = Modifier, caption: String? = null) {
    val scheme = MaterialTheme.colorScheme
    val spec = MaterialTheme.motionScheme.defaultEffectsSpec<androidx.compose.ui.graphics.Color>()
    val fill by animateColorAsState(if (selected) scheme.primary else scheme.surfaceContainerHigh, spec, label = "pill")
    val fg by animateColorAsState(if (selected) scheme.onPrimary else scheme.onSurface, spec, label = "pill-text")
    Surface(
        selected = selected,
        onClick = onPress,
        modifier = modifier.heightIn(min = 44.dp),
        shape = MaterialTheme.shapes.medium,
        color = fill,
        contentColor = fg,
    ) {
        Column(Modifier.padding(horizontal = Space.md, vertical = Space.sm), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.Center) {
            caption?.let { Text(it, style = MaterialTheme.typography.labelSmall, color = fg.copy(alpha = 0.75f)) }
            Text(label, style = MaterialTheme.typography.labelLarge.nums(), maxLines = 1, overflow = TextOverflow.Ellipsis)
        }
    }
}
