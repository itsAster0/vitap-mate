package vitapmate.features.login

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Copy
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Forward
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MailCheck
import com.composables.icons.lucide.RefreshCw
import vitapmate.core.Forwarding
import vitapmate.core.ForwardingState
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonVariant
import vitapmate.ui.Dot
import vitapmate.ui.Space
import vitapmate.ui.showToast

private const val GMAIL_FORWARDING = "https://mail.google.com/mail/u/0/#settings/fwdandpop"
private const val GMAIL_FILTERS = "https://mail.google.com/mail/u/0/#settings/filters"

/**
 * Mail forwarding with an address: the address (copy), the status, Gmail's confirmation (code
 * and link) once it arrives, the three Gmail steps with links, Check and Turn off.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ForwardingSetup(status: Forwarding, onCheck: () -> Unit, onOff: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val context = LocalContext.current
    val address = status.address.orEmpty()
    Column(
        Modifier.fillMaxWidth().background(scheme.surfaceContainerHighest, MaterialTheme.shapes.large).padding(start = Space.lg, top = Space.md, bottom = Space.md, end = Space.xs),
    ) {
        Text("Your address", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(address, Modifier.weight(1f).testTag("forwarding.address"), style = MaterialTheme.typography.bodyLarge.copy(fontFamily = FontFamily.Monospace))
            IconButton({ copy(context, "Forwarding address", address) }, Modifier.testTag("forwarding.copy")) { Icon(Lucide.Copy, "Copy the address") }
        }
    }
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
        val on = status.state == ForwardingState.ON
        Dot(size = 8.dp, color = if (on) tones.success.base else tones.warning.base)
        Text(
            when {
                status.lastOtpAt != null -> "Working · last OTP ${ago(status.lastOtpAt!!)}"
                on -> "Ready · no OTP forwarded yet"
                else -> "Waiting for Gmail's confirmation mail…"
            },
            Modifier.testTag("forwarding.status"),
            style = MaterialTheme.typography.bodyMedium,
            color = scheme.onSurfaceVariant,
        )
    }
    status.confirmation?.let { confirm ->
        Callout(Lucide.MailCheck, tones.success, confirm.requester?.let { "Gmail's confirmation for $it arrived." } ?: "Gmail's confirmation arrived.") {
            confirm.code?.let { code ->
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(code, Modifier.weight(1f).testTag("forwarding.confirm_code"), style = MaterialTheme.typography.titleLarge.copy(fontFamily = FontFamily.Monospace))
                    IconButton({ copy(context, "Gmail confirmation code", code) }) { Icon(Lucide.Copy, "Copy the code") }
                }
            }
            confirm.link?.let { link ->
                val uri = LocalUriHandler.current
                Button({ uri.openUri(link) }, Modifier.testTag("forwarding.confirm_link"), prefix = Lucide.ExternalLink) { Text("Confirm in Gmail") }
            }
        }
    }
    Step(1, "Add the address in Gmail", GMAIL_FORWARDING, "Open forwarding settings") {
        "In the Gmail where VTOP's mail arrives (usually @vitapstudent.ac.in): Add a forwarding address, paste yours, then Next → Proceed → OK. " +
            "On a phone, open the link in Chrome with ⋮ → Desktop site. Leave \"Disable forwarding\" selected."
    }
    Step(2, "Confirm it", null, null) {
        "Gmail's confirmation mail comes here, not to your inbox. Tap Check, then Confirm in Gmail or type the code into Gmail's Verify box."
    }
    Step(3, "Create the filter", GMAIL_FILTERS, "Open filters") {
        "Create a new filter with From ${status.sender}, tick Forward it to and pick your vtop-otp address, then Create filter."
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text("From", Modifier.padding(start = 40.dp, end = Space.sm), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        Text(status.sender, Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace))
        IconButton({ copy(context, "VTOP sender", status.sender) }, Modifier.testTag("forwarding.copy_sender")) { Icon(Lucide.Copy, "Copy the sender") }
    }
    Text(
        "Still asked to type the OTP? The filter's From must be exactly ${status.sender} and its action Forward to your address. Mail sent before the filter isn't forwarded.",
        style = MaterialTheme.typography.bodySmall,
        color = scheme.onSurfaceVariant,
    )
    val checking = MailForwarding.busy == "check"
    Row(horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
        Button(if (MailForwarding.busy != null) null else onCheck, Modifier.weight(1f).testTag("forwarding.check"), prefix = if (checking) null else Lucide.RefreshCw) {
            if (checking) LoadingIndicator(Modifier.size(20.dp))
            Text(if (checking) "Checking…" else "Check")
        }
        Button(if (MailForwarding.busy != null) null else onOff, Modifier.weight(1f).testTag("forwarding.off"), variant = ButtonVariant.Outline) {
            Text(if (MailForwarding.busy == "off") "Turning off…" else "Turn off", color = scheme.error)
        }
    }
}

/** One setup step: a number in a circle, a title, what to do, and a link to the page; test tags `<tag>.step<n>`. */
@Composable
internal fun Step(number: Int, title: String, link: String?, linkLabel: String?, tag: String = "forwarding", body: () -> String) {
    val scheme = MaterialTheme.colorScheme
    val uri = LocalUriHandler.current
    Row(horizontalArrangement = Arrangement.spacedBy(Space.md), modifier = Modifier.testTag("$tag.step$number")) {
        Box(Modifier.size(28.dp).background(scheme.primaryContainer, CircleShape), Alignment.Center) {
            Text("$number", style = MaterialTheme.typography.labelLarge, color = scheme.onPrimaryContainer)
        }
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            Text(body(), style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
            if (link != null && linkLabel != null) {
                TextButton({ uri.openUri(link) }, Modifier.testTag("$tag.step$number.link")) {
                    Icon(Lucide.ExternalLink, null, Modifier.size(16.dp))
                    Spacer(Modifier.size(Space.xs))
                    Text(linkLabel)
                }
            }
        }
    }
}

private fun copy(context: Context, label: String, text: String) {
    context.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText(label, text))
    showToast("Copied", label)
}

/** "just now", "5 min ago", "3 h ago", "2 d ago" for unix seconds [at]. */
fun ago(at: Long): String {
    val s = System.currentTimeMillis() / 1000 - at
    return when {
        s < 60 -> "just now"
        s < 3600 -> "${s / 60} min ago"
        s < 86_400 -> "${s / 3600} h ago"
        else -> "${s / 86_400} d ago"
    }
}
