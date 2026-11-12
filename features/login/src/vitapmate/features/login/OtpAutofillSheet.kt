package vitapmate.features.login

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import com.composables.icons.lucide.Forward
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Mail
import vitapmate.data.OtpSource
import vitapmate.data.Settings
import vitapmate.observability.Log
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.OpenHaptic
import vitapmate.ui.Space

/**
 * Settings → Account → OTP auto-fill: pick mail forwarding or Gmail (one at a time, see
 * [OtpAutofill]), then that one's setup ([ForwardingContent] or [GmailContent]). Test tags
 * `otp_autofill.*`, `forwarding.*`, `gmail.*`.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalComposeUiApi::class)
@Composable
fun OtpAutofillSheet(onDismiss: () -> Unit) {
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    val selected = OtpAutofill.selected()
    // On close, the server hears of a new pick or Gmail grant, for its saved password.
    val opened = remember { OtpAutofill.serverKey() }
    DisposableEffect(Unit) { onDispose { if (OtpAutofill.serverKey() != opened) OtpAutofill.tellServer() } }
    OpenHaptic()
    ModalBottomSheet(
        onDismiss,
        Modifier.semantics { testTagsAsResourceId = true }.testTag("otp_autofill.sheet"),
        sheet,
        containerColor = MaterialTheme.colorScheme.surfaceContainerLow,
    ) {
        Column(
            Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = Space.xl).padding(bottom = Space.xl),
            verticalArrangement = Arrangement.spacedBy(Space.lg),
        ) {
            Text("OTP auto-fill", style = MaterialTheme.typography.titleLarge)
            ConnectedChoice(
                OtpSource.entries,
                selected,
                onSelect = {
                    Log.info("ui.otp_autofill", "picked ${it.name.lowercase()}")
                    Settings.otpSource = it
                },
                tag = "otp_autofill",
                key = { it.name.lowercase() },
                label = { if (it == OtpSource.Forwarding) "Mail forwarding" else "Gmail" },
                modifier = Modifier.fillMaxWidth(),
                icon = { if (it == OtpSource.Forwarding) Lucide.Forward else Lucide.Mail },
            )
            Text(
                "Only the one picked here fills in VTOP's OTP. The other keeps its setup, so switching back needs nothing redone.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            when (selected) {
                OtpSource.Forwarding -> ForwardingContent()
                OtpSource.Gmail -> GmailContent()
            }
        }
    }
}
