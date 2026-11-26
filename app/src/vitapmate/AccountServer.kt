package vitapmate

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Link
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Smartphone
import com.composables.icons.lucide.Trash2
import com.composables.icons.lucide.TriangleAlert
import com.composables.icons.lucide.Unlink
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import vitapmate.core.LiveTopic
import vitapmate.core.LinkedPhone
import vitapmate.core.RegnoSharing
import vitapmate.core.ServerAccount
import vitapmate.core.ServerLink
import vitapmate.data.AccountWatch
import vitapmate.data.AppScope
import vitapmate.data.RegnoConsent
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.data.rememberLive
import vitapmate.features.login.MailForwarding
import vitapmate.features.login.ago
import vitapmate.ui.AppTheme
import vitapmate.ui.ProblemView
import vitapmate.ui.StatusChip
import vitapmate.ui.neutralTone

/** Account's server calls, as flows `account.<name>`. */
private val accountCall = ServerCalls("account", "Account")

/**
 * Account's server part (Rust's `server.rs`): whether the registration number is shared, and
 * once it is, the account on the server ([phase], loaded and failing in its own group, so the
 * rest of the page works offline). Hidden for the demo account.
 */
@Stable
internal class AccountServer(private val scope: CoroutineScope, val actions: ServerActions) {
    var sharing by mutableStateOf<RegnoSharing?>(null)
        private set
    var phase by mutableStateOf<ServerLoad<ServerAccount>>(ServerLoad.Loading)
        private set

    val shown get() = !AccountWatch.demo && sharing != null
    val shared get() = shown && sharing == RegnoSharing.SHARED
    val linked get() = shared && phase.ready?.link is ServerLink.Linked

    fun readSharing() = scope.launch {
        sharing = orNull { coreFlow("account.regno") { c, f -> c.regnoSharing(f) } }
        if (shared) load() else phase = ServerLoad.Loading
    }

    fun load() = scope.launch {
        if (!shared) return@launch
        phase = phase.after(accountCall("load") { c, f -> c.serverAccount(f) })
    }

    fun retry() {
        phase = ServerLoad.Loading
        load()
    }

    /** After a confirmed change: a deleted account stops sharing and clears forwarding, anything else reads again. */
    fun done(confirm: ServerConfirm) {
        if (confirm.name == "delete") {
            readSharing()
            // Deleting the server account clears forwarding.json in Rust; drop the stale status.
            AppScope.launch { MailForwarding.load("account_delete") }
        } else {
            load()
        }
    }

    /** The header's chip: linked to the server or not, once loaded. */
    @Composable
    fun chip(): (@Composable () -> Unit)? {
        val account = phase.ready?.takeIf { shared } ?: return null
        val host = rememberServer().host
        return {
            when (account.link) {
                is ServerLink.Linked -> StatusChip("Linked · $host", AppTheme.tones.success, Modifier.testTag("account.server_chip"))
                ServerLink.Unlinked -> StatusChip("Not linked", neutralTone, Modifier.testTag("account.server_chip"))
            }
        }
    }
}

@Composable
internal fun rememberAccountServer(): AccountServer {
    val scope = rememberCoroutineScope()
    val actions = rememberServerActions(accountCall)
    val server = remember(scope, actions) { AccountServer(scope, actions) }
    // Read again after each "Share your registration number?" sheet.
    val sheetOpen = RegnoConsent.pending.collectAsState().value != null
    LaunchedEffect(sheetOpen) { if (!sheetOpen) server.readSharing() }
    val change by rememberLive(LiveTopic.ACCOUNT, server.phase is ServerLoad.Ready)
    LaunchedEffect(change) { if (change > 0) server.load() }
    return server
}

/** The server groups: sharing, the link, linked phones and deleting the account. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal fun LazyListScope.accountServer(server: AccountServer) {
    val sharing = server.sharing ?: return
    if (!server.shown) return
    item {
        ListGroup("Server", footer = "The vitapmate server runs connected apps, OTP auto-fill, notifications and the server routes.") {
            RegnoSharingSetting(sharing) { server.readSharing() }
            if (server.shared) when (val phase = server.phase) {
                ServerLoad.Loading -> GroupSegment(Modifier.testTag("account.server.loading")) {
                    Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                        LoadingIndicator(Modifier.size(32.dp))
                        Text("Loading your server account…", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                }
                is ServerLoad.Failed -> GroupSegment { ProblemView(phase.problem, onRetry = server::retry, Modifier.fillMaxWidth()) }
                is ServerLoad.Ready -> ServerLinkRow(phase.value)
            }
        }
    }
    val account = server.phase.ready?.takeIf { server.shared } ?: return
    val link = account.link
    if (link is ServerLink.Linked && link.phones.isNotEmpty()) item {
        ListGroup("Linked phones", footer = "Linked phones are asked for VTOP sessions. A removed phone links again the next time it proves who you are.") {
            link.phones.forEach { phone -> PhoneRow(phone, server.actions) }
        }
    }
    item {
        ListGroup(null) {
            GroupItem(
                "Delete server account",
                "account.server.delete",
                Lucide.Trash2,
                "Keys, phones, session, password, history and the forwarding address",
                enabled = !server.actions.busy,
                tone = AppTheme.tones.danger,
                onClick = {
                    server.actions.ask(
                        ServerConfirm(
                            "Delete your server account?",
                            "The server deletes your account: every access key stops working, every phone is unlinked, and the saved session, password, options, sign-in history and forwarding address go. Gmail's forwarding to it stops working. " +
                                "Sharing your registration number turns off, so no new account is made until you turn it on again. Your data on this phone stays.",
                            "Delete",
                            "delete",
                            "The server keeps no account for you now.",
                        ) { c, f -> c.deleteServerAccount(f); true },
                    )
                },
            )
        }
    }
}

/** Linked or not, since when, and a password VTOP rejected on the server. */
@Composable
private fun ServerLinkRow(account: ServerAccount) {
    val host = rememberServer().host
    val since = account.createdAt?.let { " since ${ago(it)}" }.orEmpty()
    when (account.link) {
        is ServerLink.Linked -> GroupItem("Linked to $host", "account.server.link", Lucide.Link, "${account.registrationNumber}$since")
        ServerLink.Unlinked -> GroupItem(
            "This phone isn't linked",
            "account.server.link",
            Lucide.Unlink,
            "It links again the next time it proves who you are, such as when you open Connected apps",
        )
    }
    account.credentialsRejectedAt?.let {
        GroupItem("VTOP rejected the password saved on the server", "account.server.rejected", Lucide.TriangleAlert, ago(it), tone = AppTheme.tones.warning)
    }
}

@Composable
private fun PhoneRow(phone: LinkedPhone, actions: ServerActions) {
    val dates = "seen ${ago(phone.seenAt)} · added ${ago(phone.addedAt)}"
    GroupItem(
        phone.label ?: "Phone",
        "account.phone.${phone.id}",
        Lucide.Smartphone,
        if (phone.thisPhone) null else dates,
        below = if (!phone.thisPhone) null else {
            {
                Row(Modifier.padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                    StatusChip("This phone", AppTheme.tones.accentTone, solid = true)
                    Text(dates, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        },
        trailing = if (phone.thisPhone) null else {
            {
                IconButton(
                    {
                        actions.ask(
                            ServerConfirm(
                                "Remove ${phone.label ?: "this phone"}?",
                                "It is unlinked from your account, no longer asked for VTOP sessions, and stops getting your forwarded OTPs. Keys it made stop getting sessions from it.",
                                "Remove",
                                "remove_phone",
                                "It is no longer linked.",
                            ) { c, f -> c.removePhone(f, phone.id); true },
                        )
                    },
                    Modifier.testTag("account.phone.${phone.id}.remove"),
                    enabled = !actions.busy,
                ) { Icon(Lucide.Trash2, "Remove") }
            }
        },
    )
}
