package vitapmate.features.login

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.Copy
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Funnel
import com.composables.icons.lucide.GraduationCap
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MailCheck
import com.composables.icons.lucide.Play
import com.composables.icons.lucide.RefreshCw
import com.composables.icons.lucide.Trash2
import com.composables.icons.lucide.User
import vitapmate.core.ConfirmBy
import vitapmate.core.ConfirmFrom
import vitapmate.core.Forwarding
import vitapmate.core.ForwardingHint
import vitapmate.core.GmailConfirmation
import vitapmate.core.SetupStep
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonVariant
import vitapmate.ui.Dot
import vitapmate.ui.Space
import vitapmate.ui.heroCookie
import vitapmate.ui.neutralTone

/**
 * Mail forwarding with an address: the address (copy) and its status, three Gmail steps driven by
 * Rust's [SetupStep] (each with Gmail opened in Chrome on the college account and "Show me"
 * screens, [GuideSheet]), Gmail's confirmation with whose Gmail sent it, the typed-OTP hint
 * once on, Check and Turn off, then the phones on it ([ForwardingPhones]). Nothing here waits
 * for VTOP to send an OTP.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun ForwardingSetup(status: Forwarding, onCheck: () -> Unit, onDone: () -> Unit, onOff: () -> Unit) {
    val context = LocalContext.current
    val address: String
    val sender: String
    val confirmation: GmailConfirmation?
    val step: SetupStep?
    val collegeMail: String?
    when (status) {
        is Forwarding.Off -> return
        is Forwarding.Waiting -> {
            address = status.address
            sender = status.sender
            confirmation = status.confirmation
            step = status.step
            collegeMail = status.collegeMail
        }
        is Forwarding.On -> {
            address = status.address
            sender = status.sender
            confirmation = status.confirmation
            step = null
            collegeMail = status.collegeMail
        }
    }
    val label = statusLabel(status)
    var guide by rememberSaveable { mutableStateOf<Int?>(null) }
    AddressCard(address, label, status is Forwarding.On)
    // Rejoining: the app can't tell which steps the kept setup had done, so it shows none of them.
    if (step == SetupStep.REJOINING) {
        Callout(
            Lucide.RefreshCw,
            neutralTone,
            "Reconnecting: this phone rejoins your address the next time you're signed in to VTOP. The steps show again if anything is left to do in Gmail.",
        )
        ForwardingFeedbackNote()
        CheckAndOff(onCheck, onOff)
        return
    }
    // Done steps tick; the current one is filled; On has all three done.
    val current = when (step) {
        SetupStep.ADD_ADDRESS -> 1
        SetupStep.CONFIRM -> 2
        SetupStep.FILTER -> 3
        SetupStep.REJOINING, null -> 4
    }
    Progress(current)
    fun state(n: Int) = when {
        n < current -> StepState.Done
        n == current -> StepState.Current
        else -> StepState.Plain
    }
    Step(1, "Add the address in Gmail", null, null, state = state(1), extra = {
        if (current == 1) {
            collegeMail?.let { AccountChip(it) }
            ActionRow {
                Button({ openInChrome(context, gmailUrl(GmailPage.Forwarding, collegeMail)) }, Modifier.testTag("forwarding.step1.open"), size = vitapmate.ui.ButtonSize.Sm, prefix = Lucide.ExternalLink) { Text("Open in Chrome") }
                ShowMe { guide = 1 }
            }
        }
    }) {
        "In the Gmail where VTOP's mail arrives: Settings → Forwarding → Add a forwarding address, paste yours, then Next → Proceed. " +
            "Leave \"Disable forwarding\" selected. $DESKTOP_SITE"
    }
    // Opening the link is all the app sees: while Gmail's mail is kept, it can be opened again in
    // case the student left Gmail before tapping Confirm.
    val reopen = confirmation.link()?.takeIf { status is Forwarding.Waiting && current > 2 }
    Step(2, "Confirm it", null, null, state = state(2), trailing = reopen?.let { link ->
        {
            IconButton({ openInChrome(context, link) }, Modifier.testTag("forwarding.confirm_again")) {
                Icon(Lucide.ExternalLink, "Open Gmail's confirmation again", Modifier.size(18.dp))
            }
        }
    }, extra = {
        if (confirmation != null && current == 2) Confirmation(confirmation, collegeMail)
        if (current == 2) ShowMe { guide = 2 }
    }) {
        // Says what the app saw: the link opened, not that Gmail took it.
        when {
            current <= 2 -> "Gmail mails a confirmation to your address. It shows up here on its own, then it's one tap."
            reopen != null -> "Link opened. Left Gmail before tapping Confirm? Open it again."
            else -> "Link opened."
        }
    }
    Step(3, "Create the filter", null, null, state = state(3), last = true, extra = {
        if (current == 3) {
            ActionRow {
                Button({ openInChrome(context, gmailUrl(GmailPage.Filters, collegeMail)) }, Modifier.testTag("forwarding.step3.open"), size = vitapmate.ui.ButtonSize.Sm, prefix = Lucide.Funnel) { Text("Open filters") }
                ShowMe { guide = 3 }
            }
            // A local write: it never waits for a check running.
            Button(
                onDone,
                Modifier.padding(top = Space.sm).testTag("forwarding.done"),
                ButtonVariant.Outline,
                vitapmate.ui.ButtonSize.Sm,
                prefix = Lucide.Check,
            ) { Text("I've created it") }
        }
    }) {
        "Reload Gmail first. Then a new filter with From $sender, tick Forward it to and pick your address."
    }
    if (status is Forwarding.Waiting && status.step != SetupStep.REJOINING) {
        Text(
            "That's it after step 3: forwarding is on. Nothing waits for VTOP to send an OTP; when one comes through, you'll see it here.",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
    (status as? Forwarding.On)?.confirmation?.let { Confirmation(it, collegeMail, canIgnore = true) }
    (status as? Forwarding.On)?.hint?.let { TypedOtpHint(it, sender, collegeMail) { guide = 3 } }
    guide?.let { n ->
        val titles = mapOf(1 to "Add the address", 2 to "Confirm it", 3 to "Create the filter")
        GuideSheet(n, titles.getValue(n), address, sender, label) { guide = null }
    }
    ForwardingFeedbackNote()
    CheckAndOff(onCheck, onOff)
    ForwardingPhones()
}

/** On a phone, Gmail opens its mobile site, which has no Settings. */
private const val DESKTOP_SITE = "On a phone, first turn on Chrome ⋮ → Desktop site: Gmail's mobile site has no Settings."

/** A step's buttons, wrapping onto a second line on a narrow screen or with large text. */
@Composable
private fun ActionRow(content: @Composable () -> Unit) {
    FlowRow(
        Modifier.padding(top = Space.sm),
        horizontalArrangement = Arrangement.spacedBy(Space.sm),
        verticalArrangement = Arrangement.spacedBy(Space.xs),
        itemVerticalAlignment = Alignment.CenterVertically,
    ) { content() }
}

/** Check and Turn off, side by side. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun CheckAndOff(onCheck: () -> Unit, onOff: () -> Unit) {
    val tones = AppTheme.tones
    val checking = MailForwarding.busy == ForwardingOp.Check
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
            Text(if (MailForwarding.busy == ForwardingOp.TurnOff) "Turning off…" else "Turn off")
        }
    }
}

/** The status line on the address card, as Rust's state says it. */
private fun statusLabel(status: Forwarding): String = when (status) {
    is Forwarding.Off -> "Off"
    is Forwarding.On -> status.lastOtpAt?.let { "On · last OTP ${ago(it)}" } ?: "On · no OTP forwarded yet"
    is Forwarding.Waiting -> when (status.step) {
        SetupStep.ADD_ADDRESS -> "Waiting for Gmail's confirmation…"
        SetupStep.CONFIRM -> "Confirm in Gmail"
        SetupStep.FILTER -> "Create the filter"
        SetupStep.REJOINING -> "Reconnecting once you're signed in"
    }
}

@Composable
private fun AddressCard(address: String, label: String, on: Boolean) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val context = LocalContext.current
    val accent = tones.accentTone
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
        val tone = if (on) tones.success else neutralTone
        Row(
            Modifier.padding(top = Space.xs).background(tone.subtle, CircleShape).padding(horizontal = Space.sm, vertical = 3.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Space.xs),
        ) {
            Dot(size = 8.dp, color = if (on) tones.success.base else tones.warning.base)
            Text(label, Modifier.testTag("forwarding.status"), style = MaterialTheme.typography.labelMedium, color = tone.onSubtle)
        }
    }
}

/** Three segments for the three steps, done ones in the success colour; [current] 4 is all done. */
@Composable
private fun Progress(current: Int) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    Row(Modifier.testTag("forwarding.progress"), horizontalArrangement = Arrangement.spacedBy(Space.xs), verticalAlignment = Alignment.CenterVertically) {
        for (n in 1..3) {
            val color = when {
                n < current -> tones.success.base
                n == current -> scheme.primary
                else -> scheme.surfaceContainerHighest
            }
            Box(Modifier.weight(1f).height(6.dp).background(color, CircleShape))
        }
        Text("${current.coerceAtMost(3)} of 3", Modifier.padding(start = Space.sm), style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
    }
}

@Composable
private fun AccountChip(mail: String) {
    val scheme = MaterialTheme.colorScheme
    Row(
        Modifier.padding(top = Space.sm).background(scheme.surfaceContainerLow, CircleShape).padding(horizontal = Space.sm, vertical = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Space.xs),
    ) {
        Icon(Lucide.GraduationCap, null, Modifier.size(14.dp), tint = scheme.onSurfaceVariant)
        Text(mail, style = MaterialTheme.typography.labelMedium)
    }
}

@Composable
private fun ShowMe(onClick: () -> Unit) {
    TextButton(onClick, Modifier.testTag("forwarding.guide.open")) {
        Icon(Lucide.Play, null, Modifier.size(16.dp))
        Spacer(Modifier.size(Space.xs))
        Text("Show me")
    }
}

/**
 * Gmail's confirmation: who asked, and one tap to confirm (its link opens in Chrome and moves the
 * sheet on). A Gmail that isn't the college one gets [WrongAccount] first, as VTOP's OTPs never
 * reach it.
 */
@Composable
private fun Confirmation(confirm: GmailConfirmation, collegeMail: String?, canIgnore: Boolean = false) {
    val tones = AppTheme.tones
    val context = LocalContext.current
    var anyway by rememberSaveable(confirm.sentAt) { mutableStateOf(false) }
    var showCode by rememberSaveable(confirm.sentAt) { mutableStateOf(false) }
    // No browser took the link: it was copied, and the student says when they confirmed.
    var noBrowser by rememberSaveable(confirm.sentAt) { mutableStateOf(false) }
    val wrong = confirm.from == ConfirmFrom.UNKNOWN || confirm.from == ConfirmFrom.OWN_OTHER
    if (wrong && !anyway) return WrongAccount(confirm.requester, collegeMail, onAnyway = { anyway = true }, onIgnore = MailForwarding::ignoreConfirm)
    val said = when (confirm.from) {
        ConfirmFrom.COLLEGE_INBOX -> "From your college Gmail${confirm.requester?.let { "\n$it" } ?: ""}"
        ConfirmFrom.NOT_SAID -> "Gmail's confirmation arrived. Confirm only if you just added this address in Gmail."
        ConfirmFrom.OWN_OTHER, ConfirmFrom.UNKNOWN -> "Gmail's confirmation for ${confirm.requester ?: "an address"} arrived."
    }
    val tone = if (confirm.from == ConfirmFrom.COLLEGE_INBOX) tones.success else tones.warning
    val link = confirm.link()
    val code = when (val by = confirm.confirmBy) {
        is ConfirmBy.Code -> by.code
        is ConfirmBy.CodeAndLink -> by.code
        is ConfirmBy.Link, ConfirmBy.Neither -> null
    }
    Box(Modifier.padding(top = Space.sm)) {
        Callout(if (confirm.from == ConfirmFrom.COLLEGE_INBOX) Lucide.GraduationCap else Lucide.MailCheck, tone, said) {
            if (code != null && (link == null || showCode)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(code, Modifier.weight(1f).testTag("forwarding.confirm_code"), style = MaterialTheme.typography.titleLarge.copy(fontFamily = FontFamily.Monospace))
                    IconButton({ copy(context, "Gmail confirmation code", code) }) { Icon(Lucide.Copy, "Copy the code") }
                }
            }
            if (noBrowser) {
                Text(
                    "No browser opened, so the link was copied. Paste it into a browser, confirm there, then tap I've confirmed.",
                    Modifier.testTag("forwarding.confirm_copied"),
                    style = MaterialTheme.typography.bodySmall,
                    color = tone.onSubtle,
                )
            }
            FlowRow(horizontalArrangement = Arrangement.spacedBy(Space.xs), itemVerticalAlignment = Alignment.CenterVertically) {
                if (link != null) {
                    Button(
                        { if (openInChrome(context, link)) MailForwarding.openedConfirm() else noBrowser = true },
                        Modifier.testTag("forwarding.confirm_link"),
                        size = vitapmate.ui.ButtonSize.Sm,
                        prefix = Lucide.Check,
                    ) { Text("Confirm in Gmail") }
                    if (code != null && !showCode) TextButton({ showCode = true }) { Text("Use the code") }
                }
                // With no link to open, the student says when they confirmed in Gmail.
                if (link == null || showCode || noBrowser) TextButton({ MailForwarding.openedConfirm() }, Modifier.testTag("forwarding.confirmed")) { Text("I've confirmed") }
                if (canIgnore) TextButton({ MailForwarding.ignoreConfirm() }, Modifier.testTag("forwarding.confirm_ignore")) { Text("Dismiss") }
            }
        }
    }
}

/** The confirm link in Gmail's mail, if it has one. */
internal fun GmailConfirmation?.link(): String? = when (val by = this?.confirmBy) {
    is ConfirmBy.Link -> by.link
    is ConfirmBy.CodeAndLink -> by.link
    is ConfirmBy.Code, ConfirmBy.Neither, null -> null
}

/** Gmail's confirmation came from a Gmail that isn't the college one: VTOP's OTPs never reach it. */
@Composable
private fun WrongAccount(requester: String?, collegeMail: String?, onAnyway: () -> Unit, onIgnore: () -> Unit) {
    val tones = AppTheme.tones
    val context = LocalContext.current
    Column(Modifier.padding(top = Space.sm).testTag("forwarding.wrong_account"), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
        Callout(Lucide.User, tones.warning, "That's not your college Gmail${requester?.let { ": $it" } ?: ""}. VTOP sends OTPs to your college mail, so this Gmail never gets them.") {
            FlowRow(horizontalArrangement = Arrangement.spacedBy(Space.xs), itemVerticalAlignment = Alignment.CenterVertically) {
                Button(
                    { openInChrome(context, collegeGmailUrl(GmailPage.Forwarding, collegeMail)) },
                    Modifier.testTag("forwarding.wrong_account.open"),
                    size = vitapmate.ui.ButtonSize.Sm,
                    prefix = Lucide.ExternalLink,
                ) { Text("Open college Gmail") }
                TextButton(onAnyway, Modifier.testTag("forwarding.wrong_account.anyway")) { Text("It's right, confirm anyway") }
                TextButton(onIgnore, Modifier.testTag("forwarding.wrong_account.ignore")) { Text("Ignore") }
            }
        }
        Callout(Lucide.Trash2, tones.lab, "Also remove the address from that Gmail's forwarding settings, so it doesn't sit there unused.")
    }
}

/** The student typed an OTP by hand while forwarding was on: worth a look at the filter. Changes nothing. */
@Composable
private fun TypedOtpHint(hint: ForwardingHint, sender: String, collegeMail: String?, onShowMe: () -> Unit) {
    val tones = AppTheme.tones
    val scheme = MaterialTheme.colorScheme
    val context = LocalContext.current
    val whenTyped = when (hint) {
        is ForwardingHint.TypedOtp -> ago(hint.at)
    }
    Column(Modifier.testTag("forwarding.hint"), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
        Callout(Lucide.Funnel, tones.warning, "Typed the OTP yourself $whenTyped? It may not have been forwarded. Worth a quick look at the filter.") {
            FlowRow(horizontalArrangement = Arrangement.spacedBy(Space.xs), itemVerticalAlignment = Alignment.CenterVertically) {
                ShowMe(onShowMe)
                TextButton({ MailForwarding.dismissHint() }, Modifier.testTag("forwarding.hint.dismiss")) { Text("Dismiss") }
            }
        }
        val checks = listOf(
            HintCheck("The filter's From is exactly", sender, GmailPage.Filters, "sender", copyText = sender),
            HintCheck("It forwards to your address", "Forward it to → your address", GmailPage.Filters, "forward_to"),
            HintCheck("It's in your college Gmail", collegeMail ?: "The Gmail VTOP mails", GmailPage.Forwarding, "account"),
            HintCheck("Forwarding is still verified", "Gmail drops it if the address changed", GmailPage.Forwarding, "verified"),
        )
        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
            checks.forEach { check ->
                Row(
                    Modifier.fillMaxWidth().clip(MaterialTheme.shapes.small).background(scheme.surfaceContainerLow)
                        .padding(horizontal = Space.md, vertical = Space.sm),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(Modifier.weight(1f)) {
                        Text(check.title, style = MaterialTheme.typography.bodyMedium)
                        Text(check.detail, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
                    }
                    check.copyText?.let { text ->
                        IconButton({ copy(context, "VTOP's sender", text) }, Modifier.testTag("forwarding.hint.${check.tag}.copy")) { Icon(Lucide.Copy, "Copy VTOP's sender") }
                    }
                    IconButton({ openInChrome(context, gmailUrl(check.page, collegeMail)) }, Modifier.testTag("forwarding.hint.${check.tag}.open")) {
                        Icon(Lucide.ChevronRight, "Open in Gmail: ${check.title}")
                    }
                }
            }
        }
    }
}

/** One thing to look at in Gmail for the typed-OTP hint, with the page that shows it and a value to copy. */
private data class HintCheck(val title: String, val detail: String, val page: GmailPage, val tag: String, val copyText: String? = null)

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
    trailing: (@Composable () -> Unit)? = null,
    extra: @Composable () -> Unit = {},
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
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(title, Modifier.weight(1f), style = MaterialTheme.typography.titleSmall)
                trailing?.invoke()
            }
            Text(body(), style = MaterialTheme.typography.bodyMedium, color = scheme.onSurfaceVariant)
            if (link != null && linkLabel != null) {
                TextButton({ uri.openUri(link) }, Modifier.testTag("$tag.step$number.link")) {
                    Icon(Lucide.ExternalLink, null, Modifier.size(16.dp))
                    Spacer(Modifier.size(Space.xs))
                    Text(linkLabel)
                }
            }
            extra()
        }
    }
}

internal fun copy(context: Context, label: String, text: String) {
    context.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText(label, text))
    MailForwarding.note("Copied", label)
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
