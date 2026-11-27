package vitapmate.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sparkles

/** Opens Settings at the insights toggle; the app provides it. */
val LocalOpenInsights = staticCompositionLocalOf<(() -> Unit)?> { null }

/**
 * A small centred pill where a screen's insights would be, while they're off: VTOP's numbers
 * only. Taps open Settings at the toggle ([LocalOpenInsights]).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun InsightsHint(modifier: Modifier = Modifier) {
    val open = LocalOpenInsights.current
    Box(modifier.fillMaxWidth(), contentAlignment = Alignment.Center) {
        Surface(
            onClick = { open?.invoke() },
            enabled = open != null,
            shape = CircleShape,
            color = MaterialTheme.colorScheme.surfaceContainerHigh,
            contentColor = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.testTag("insights.hint"),
        ) {
            Row(Modifier.padding(start = Space.md, top = Space.sm, end = Space.sm, bottom = Space.sm), verticalAlignment = Alignment.CenterVertically) {
                Icon(Lucide.Sparkles, null, Modifier.size(16.dp), tint = MaterialTheme.colorScheme.primary)
                Spacer(Modifier.width(Space.sm))
                Text("Turn on insights in Settings for more", style = MaterialTheme.typography.labelLarge)
                if (open != null) Icon(Lucide.ChevronRight, null, Modifier.padding(start = Space.xs).size(16.dp))
            }
        }
    }
}
