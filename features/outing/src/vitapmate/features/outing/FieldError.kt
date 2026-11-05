package vitapmate.features.outing

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CircleAlert
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Space

/** A field's problem in the error colour, or nothing; it springs in and out. */
@Composable
internal fun FieldError(message: String?, modifier: Modifier = Modifier) {
    AnimatedVisibility(message != null, modifier, enter = fadeIn() + expandVertically(), exit = fadeOut() + shrinkVertically()) {
        Row(Modifier.padding(top = Space.xs + 2.dp, start = 2.dp)) {
            Icon(Lucide.CircleAlert, null, Modifier.padding(top = 2.dp).size(12.dp), tint = MaterialTheme.colorScheme.error)
            Spacer(Modifier.width(Space.xs + 2.dp))
            Text(message.orEmpty(), style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.error)
        }
    }
}
