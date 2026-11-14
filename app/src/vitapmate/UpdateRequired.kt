package vitapmate

import androidx.activity.compose.LocalActivity
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import com.composables.icons.lucide.ArrowUp
import com.composables.icons.lucide.Lucide

/**
 * Update priority 5: shown in place of the whole app, with no nav bar and no close (Back leaves
 * the app). "Update" opens Play's immediate update; backing out of it comes back here.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun UpdateRequired() {
    val scheme = MaterialTheme.colorScheme
    val activity = LocalActivity.current
    Column(
        Modifier.fillMaxSize().background(scheme.background).safeDrawingPadding().padding(start = 24.dp, end = 24.dp, top = 28.dp, bottom = 24.dp).testTag("update.required"),
    ) {
        Box(Modifier.padding(top = 22.dp, bottom = 24.dp).size(132.dp).background(scheme.primary, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
            Icon(Lucide.ArrowUp, null, Modifier.size(58.dp), tint = scheme.onPrimary)
        }
        Text("Update to keep using vitapmate", style = MaterialTheme.typography.displaySmallEmphasized.copy(lineHeight = 1.04.em))
        Text(
            "This version is no longer supported. Updating takes about a minute and keeps everything you have saved.",
            Modifier.padding(top = 12.dp),
            style = MaterialTheme.typography.bodyLarge,
            color = scheme.onSurfaceVariant,
        )
        Spacer(Modifier.weight(1f))
        Button(
            { activity?.let(PlayUpdate::updateNow) },
            modifier = Modifier.fillMaxWidth().heightIn(min = 56.dp).testTag("update.required.start"),
            shapes = ButtonDefaults.shapes(),
        ) { Text("Update", style = MaterialTheme.typography.titleMediumEmphasized) }
        Text(
            "Opens Google Play",
            Modifier.fillMaxWidth().padding(top = 12.dp),
            style = MaterialTheme.typography.labelMedium,
            color = scheme.onSurfaceVariant,
            textAlign = TextAlign.Center,
        )
    }
}
