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
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.text.font.FontWeight
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.Check
import vitapmate.ui.heroCookie
import vitapmate.ui.neutralTone
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.OutlinedButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
    val accent = tones.accentTone
    val on = status.state == ForwardingState.ON
    Column(
        Modifier.fillMaxWidth().clip(MaterialTheme.shapes.extraLarge).background(accent.subtle).heroCookie()
            .padding(start = Space.lg, top = Space.md, bottom = Space.lg, end = Space.xs),
    ) {
        Text("Your address", style = MaterialTheme.typography.labelMedium, color = accent.onSubtle)
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                address,
                Modifier.weight(1f).testTag("forwarding.address"),
                style = MaterialTheme.typography.bodyLarge.copy(fontFamily = FontFamily.Monospace, fontWeight = FontWeight.SemiBold),
                color = scheme.onSurface,
            )
            IconButton({ copy(context, "Forwarding address", address) }, Modifier.testTag("forwarding.copy")) { Icon(Lucide.Copy, "Copy the address", tint = accent.onSubtle) }
        }
        // Waiting in the warning tone, ready in the neutral one, working in success.
        val (label, tone) = when {
            status.lastOtpAt != null -> "Working · last OTP ${ago(status.lastOtpAt!!)}" to tones.success
            on -> "Ready · no OTP forwarded yet" to neutralTone
            else -> "Waiting for Gmail's confirmation mail…" to tones.warning
        }
        Row(
            Modifier.padding(top = Space.xs).background(tone.subtle, CircleShape).padding(horizontal = Space.sm, vertical = 3.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Space.xs),
        ) {
            Dot(size = 8.dp, color = if (on) tones.success.base else tones.warning.base)
            Text(label, Modifier.testTag("forwarding.status"), style = MaterialTheme.typography.labelMedium, color = tone.onSubtle)
        }
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
    // Ticks only for what the app can tell: Gmail's confirmation (or forwarding on) means steps 1
    // and 2 are done, a forwarded OTP means the filter works too.
    val added = on || status.confirmation != null
    val filtered = status.lastOtpAt != null
    fun state(done: Boolean, current: Boolean) = if (done) StepState.Done else if (current) StepState.Current else StepState.Plain
    Step(1, "Add the address in Gmail", GMAIL_FORWARDING, "Open forwarding settings", state = state(added, true)) {
        "In the Gmail where VTOP's mail arrives (usually @vitapstudent.ac.in): Add a forwarding address, paste yours, then Next → Proceed → OK. " +
            "On a phone, open the link in Chrome with ⋮ → Desktop site. Leave \"Disable forwarding\" selected."
    }
    Step(2, "Confirm it", null, null, state = state(added, false)) {
        "Gmail's confirmation mail comes here, not to your inbox. Tap Check, then Confirm in Gmail or type the code into Gmail's Verify box."
    }
    Step(3, "Create the filter", GMAIL_FILTERS, "Open filters", state = state(filtered, added), last = true) {
        "Reload Gmail first. Then create a new filter with From ${status.sender}, tick Forward it to and pick your address, then Create filter."
    }
    var guide by remember { mutableStateOf(false) }
    OutlinedButton({ guide = true }, Modifier.fillMaxWidth().testTag("forwarding.guide.open")) {
        Icon(Lucide.BookOpen, null, Modifier.size(18.dp))
        Spacer(Modifier.size(Space.sm))
        Text("Step-by-step guide with screenshots")
    }
    if (guide) ForwardingGuide(status.sender) { guide = false }
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
        // Turn off in the danger tint, beside Check.
        androidx.compose.material3.Button(
            onClick = onOff,
            modifier = Modifier.weight(1f).defaultMinSize(minHeight = 44.dp).testTag("forwarding.off"),
            enabled = MailForwarding.busy == null,
            shapes = ButtonDefaults.shapes(),
            colors = ButtonDefaults.buttonColors(containerColor = tones.danger.subtle, contentColor = tones.danger.onSubtle),
        ) {
            Text(if (MailForwarding.busy == "off") "Turning off…" else "Turn off")
        }
    }
}

/** How far a setup step is, when the app can tell: [Done] shows a tick, [Current] a filled number. */
internal enum class StepState { Done, Current, Plain }

/**
 * One setup step on a rail: a number mark (a tick once [state] is done), a title, what to do,
 * and a link to the page; the rail runs on to the next step unless [last]. Test tags `<tag>.step<n>`.
 */
@Composable
internal fun Step(
    number: Int,
    title: String,
    link: String?,
    linkLabel: String?,
    tag: String = "forwarding",
    state: StepState = StepState.Plain,
    last: Boolean = false,
    body: () -> String,
) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val uri = LocalUriHandler.current
    val rail = scheme.surfaceContainerHighest
    Row(
        horizontalArrangement = Arrangement.spacedBy(Space.md),
        modifier = Modifier.testTag("$tag.step$number").drawBehind {
            // From under the mark down through the gap to the next step.
            if (!last) {
                val x = 15.dp.toPx()
                drawLine(rail, Offset(x, 32.dp.toPx()), Offset(x, size.height + Space.lg.toPx()), 2.dp.toPx())
            }
        },
    ) {
        val (fill, shape) = when (state) {
            StepState.Done -> tones.success.base to CircleShape
            StepState.Current -> scheme.primary to RoundedCornerShape(10.dp)
            StepState.Plain -> scheme.secondaryContainer to CircleShape
        }
        Box(Modifier.size(30.dp).background(fill, shape), Alignment.Center) {
            when (state) {
                StepState.Done -> Icon(Lucide.Check, "Done", Modifier.size(16.dp), tint = scheme.surface)
                StepState.Current -> Text("$number", style = MaterialTheme.typography.labelLarge, color = scheme.onPrimary)
                StepState.Plain -> Text("$number", style = MaterialTheme.typography.labelLarge, color = scheme.onSecondaryContainer)
            }
        }
        Column(Modifier.weight(1f).padding(top = 4.dp)) {
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
