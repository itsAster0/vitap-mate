package vitapmate

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import com.composables.icons.lucide.Forward
import com.composables.icons.lucide.Lucide
import vitapmate.core.ForwardingState
import vitapmate.features.login.ForwardingSheet
import vitapmate.features.login.MailForwarding
import vitapmate.features.login.ago

/** Settings → Account → Mail forwarding: the status; tapping opens [ForwardingSheet]. */
@Composable
fun ForwardingSetting() {
    var open by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(Unit) { MailForwarding.load("settings") }
    val status = MailForwarding.status
    val hint = when (status?.state) {
        null -> "…"
        ForwardingState.OFF -> "Off · OTPs are typed by hand"
        ForwardingState.WAITING -> "Finish setting up in Gmail"
        ForwardingState.ON -> status.lastOtpAt?.let { "On · last OTP ${ago(it)}" } ?: "On · OTPs fill in by themselves"
    }
    GroupItem("Mail forwarding", "settings.forwarding", Lucide.Forward, hint, onClick = { open = true }, trailing = { Chevron() })
    if (open) ForwardingSheet { open = false }
}
