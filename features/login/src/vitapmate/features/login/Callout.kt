package vitapmate.features.login

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import vitapmate.ui.Space
import vitapmate.ui.Tone

/** A note on a tone's quiet fill with an icon; [extra] goes under the text. */
@Composable
internal fun Callout(icon: ImageVector, tone: Tone, text: String, extra: @Composable () -> Unit = {}) {
    Column(Modifier.fillMaxWidth().background(tone.subtle, MaterialTheme.shapes.medium).padding(Space.md)) {
        Row(horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
            Icon(icon, null, Modifier.size(18.dp), tint = tone.base)
            Text(text, style = MaterialTheme.typography.bodySmall, color = tone.onSubtle)
        }
        Spacer(Modifier.height(Space.xs))
        extra()
    }
}
