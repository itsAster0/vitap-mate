package vitapmate.features.login

import vitapmate.ui.OpenHaptic
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import vitapmate.core.Forwarding
import vitapmate.core.ForwardingState
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.StatusChip

/** While the address waits for Gmail's confirmation, the inbox is checked this often. */
private const val WAITING_POLL_MS = 5_000L

/**
 * Settings → Account → Mail forwarding (the Flutter app's `MailForwardingPage`): off, a button
 * that makes (or joins) the student's address; then the address with a copy button, the status,
 * Gmail's confirmation once it arrives, the three Gmail steps, Check and Turn off. While waiting
 * for Gmail's mail the inbox is checked every 5 s. Test tags `forwarding.*`.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
fun ForwardingSheet(onDismiss: () -> Unit) {
    val scope = rememberCoroutineScope()
    val status = MailForwarding.status
    var confirmOff by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        flow("forwarding.open") { Log.info("ui.forwarding", "open", it, mapOf("state" to "${status?.state}")) }
        MailForwarding.load("open")
    }
    LaunchedEffect(status?.state) {
        while (MailForwarding.status?.state == ForwardingState.WAITING) {
            MailForwarding.check(quiet = true)
            delay(WAITING_POLL_MS)
        }
    }
    val sheet = rememberBottomSheetState(
        initialValue = SheetValue.Hidden,
        enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded),
    )
    OpenHaptic()
    ModalBottomSheet(
        onDismiss,
        Modifier.semantics { testTagsAsResourceId = true }.testTag("forwarding.sheet"),
        sheet,
        containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
    ) {
        Column(
            Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = Space.xl).padding(bottom = Space.xl),
            verticalArrangement = Arrangement.spacedBy(Space.lg),
        ) {
            Header(status)
            when {
                status == null -> Box(Modifier.fillMaxWidth().padding(Space.xl), Alignment.Center) { LoadingIndicator() }
                status.state == ForwardingState.OFF -> ForwardingIntro(MailForwarding::setUp)
                else -> ForwardingSetup(status, onCheck = { scope.launch { MailForwarding.check() } }, onOff = { confirmOff = true })
            }
        }
    }
    if (confirmOff) {
        AlertDialog(
            onDismissRequest = { confirmOff = false },
            title = { Text("Turn off mail forwarding?") },
            text = { Text("This phone stops getting forwarded OTPs. If no other phone uses the address, it's deleted and mail sent to it is rejected; then also delete the Gmail filter.") },
            confirmButton = {
                TextButton(
                    {
                        confirmOff = false
                        MailForwarding.turnOff()
                    },
                    Modifier.testTag("forwarding.off.confirm"),
                ) { Text("Turn off", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = { TextButton({ confirmOff = false }) { Text("Cancel") } },
        )
    }
}

@Composable
private fun Header(status: Forwarding?) {
    val tones = AppTheme.tones
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Space.md)) {
        Text("Mail forwarding", Modifier.weight(1f), style = MaterialTheme.typography.titleLarge)
        when (status?.state) {
            ForwardingState.ON -> StatusChip("On", tones.success, Modifier.testTag("forwarding.state"))
            ForwardingState.WAITING -> StatusChip("Setting up", tones.warning, Modifier.testTag("forwarding.state"))
            ForwardingState.OFF -> StatusChip("Off", tones.accentTone, Modifier.testTag("forwarding.state"))
            null -> {}
        }
    }
}
