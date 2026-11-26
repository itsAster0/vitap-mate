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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.composables.icons.lucide.Forward
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.core.Forwarding
import vitapmate.core.LiveTopic
import vitapmate.core.SetupStep
import vitapmate.data.Foreground
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
    var confirmOff by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        MailForwarding.feedback = null
        flow("forwarding.open") { Log.info("ui.forwarding", "open", it, mapOf("state" to "${status?.let { it::class.simpleName?.uppercase() }}")) }
        MailForwarding.load("open")
        // Waiting is checked by the watch below; a hint may be stale since the OTP that raised it.
        if (MailForwarding.status is Forwarding.On && MailForwarding.status.worthQuietCheck) MailForwarding.check(quiet = true)
    }
    // Back from Chrome (Gmail's confirm link or the filter): the inbox may have changed while the
    // live socket was closed in the background.
    LaunchedEffect(Unit) {
        var seen = Foreground.resumes
        snapshotFlow { Foreground.resumes }.collect { resumes ->
            if (resumes == seen) return@collect
            seen = resumes
            if (MailForwarding.status.worthQuietCheck) MailForwarding.check(quiet = true)
        }
    }
    // Keyed on the address: a rejoin onto a new one starts a fresh watch. Not while rejoining,
    // where the server doesn't know this phone on the address yet.
    val watched = status.watchedAddress
    LaunchedEffect(watched) {
        if (watched == null) return@LaunchedEffect
        // Checks again when the server says the inbox changed; every few seconds only while the
        // live socket is down.
        val wake = LiveWake(this, LiveTopic.MAIL_INBOX)
        try {
            while (MailForwarding.status.watchedAddress == watched) {
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

/** The address the sheet watches for Gmail's mail: while waiting, and not while rejoining. */
internal val Forwarding?.watchedAddress: String?
    get() = (this as? Forwarding.Waiting)?.takeIf { it.step != SetupStep.REJOINING }?.address

/** Whether coming back to the sheet is worth a quiet check: Gmail's mail awaited, a newer confirmation, or a typed-OTP hint shown. */
internal val Forwarding?.worthQuietCheck: Boolean
    get() = watchedAddress != null || (this as? Forwarding.On)?.let { it.hint != null || it.confirmation != null } == true
