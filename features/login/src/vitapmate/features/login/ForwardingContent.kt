package vitapmate.features.login

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.composables.icons.lucide.Forward
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.core.Forwarding
import vitapmate.core.LiveTopic
import vitapmate.data.LiveWake
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.Space

/** While the address waits for Gmail's confirmation, the inbox is checked this often. */
private const val WAITING_POLL_MS = 5_000L

/**
 * Mail forwarding in the OTP auto-fill sheet (the Flutter app's `MailForwardingPage`): off, a
 * button that makes (or joins) the student's address; then the address with a copy button, the
 * status, Gmail's confirmation once it arrives, the three Gmail steps, Check and Turn off. While
 * waiting for Gmail's mail the inbox is checked every 5 s. Test tags `forwarding.*`.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ForwardingContent() {
    val scope = rememberCoroutineScope()
    val status = MailForwarding.status
    var confirmOff by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        MailForwarding.error = null
        flow("forwarding.open") { Log.info("ui.forwarding", "open", it, mapOf("state" to "${status?.let { it::class.simpleName?.uppercase() }}")) }
        MailForwarding.load("open")
    }
    LaunchedEffect(status is Forwarding.Waiting) {
        if (MailForwarding.status !is Forwarding.Waiting) return@LaunchedEffect
        // Checks again when the server says the inbox changed; every few seconds only while the
        // live socket is down.
        val wake = LiveWake(this, LiveTopic.MAIL_INBOX)
        try {
            while (MailForwarding.status is Forwarding.Waiting) {
                MailForwarding.check(quiet = true)
                wake.next(WAITING_POLL_MS)
            }
        } finally {
            wake.stop()
        }
    }
    when (status) {
        null -> Box(Modifier.fillMaxWidth().padding(Space.xl), Alignment.Center) { LoadingIndicator() }
        is Forwarding.Off -> ForwardingIntro(MailForwarding::setUp)
        is Forwarding.Waiting, is Forwarding.On -> ForwardingSetup(
            status,
            onCheck = { scope.launch { MailForwarding.check() } },
            onDone = MailForwarding::markDone,
            onOff = { confirmOff = true },
        )
    }
    if (confirmOff) {
        ForwardingConfirmSheet(
            onDismiss = { confirmOff = false },
            icon = Lucide.Forward,
            title = "Turn off forwarding?",
            body = "This phone stops getting forwarded OTPs. If no other phone uses the address, it's deleted and mail sent to it is rejected.",
            confirmLabel = "Turn off",
            confirmTag = "forwarding.off.confirm",
            dismissTag = "forwarding.off.cancel",
            sheetTag = "forwarding.off.sheet",
            target = status.cleanupTarget(),
            onConfirm = {
                confirmOff = false
                MailForwarding.turnOff()
            },
        )
    }
}
