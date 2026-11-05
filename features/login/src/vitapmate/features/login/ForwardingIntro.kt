package vitapmate.features.login

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.AtSign
import com.composables.icons.lucide.Forward
import com.composables.icons.lucide.Funnel
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Smartphone
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.Space

/** Mail forwarding while off: what it does, the privacy note and "Create my address". */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ForwardingIntro(onSetUp: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Text(
        "Gmail forwards VTOP's OTP mail to a private address, and the code fills in here by itself. About 2 minutes, once.",
        style = MaterialTheme.typography.bodyMedium,
        color = scheme.onSurfaceVariant,
    )
    Column(verticalArrangement = Arrangement.spacedBy(Space.md)) {
        Point(Lucide.AtSign, "You get your own address")
        Point(Lucide.Funnel, "A Gmail filter forwards only VTOP's OTP mail")
        Point(Lucide.Smartphone, "The code fills in while the OTP prompt is open")
    }
    Callout(Lucide.Lock, AppTheme.tones.success, "Only VTOP's OTP mail is relayed. The server locks each code to this phone's key as it arrives and keeps nothing it can read.")
    Text(
        "Already set up in the other vitapmate app? This phone joins that address and Gmail needs no changes.",
        style = MaterialTheme.typography.bodySmall,
        color = scheme.onSurfaceVariant,
    )
    val busy = MailForwarding.busy == "setup"
    Button(if (busy) null else onSetUp, Modifier.fillMaxWidth().testTag("forwarding.setup"), prefix = if (busy) null else Lucide.Forward) {
        if (busy) LoadingIndicator(Modifier.size(20.dp))
        Text(if (busy) "Setting up…" else "Create my address")
    }
}

@Composable
private fun Point(icon: ImageVector, text: String) {
    val lab = AppTheme.tones.lab
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Space.md)) {
        Box(Modifier.size(32.dp).background(lab.subtle, MaterialTheme.shapes.small), Alignment.Center) {
            Icon(icon, null, Modifier.size(16.dp), tint = lab.onSubtle)
        }
        Text(text, style = MaterialTheme.typography.bodyMedium)
    }
}
