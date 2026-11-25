package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.toShape
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarRange
import com.composables.icons.lucide.Cookie
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.LogOut
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Mail
import com.composables.icons.lucide.Server
import com.composables.icons.lucide.Smartphone
import com.composables.icons.lucide.Trash2
import com.composables.icons.lucide.TriangleAlert
import com.composables.icons.lucide.Unplug
import com.composables.icons.lucide.User
import com.composables.icons.lucide.Zap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.AccessKey
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.core.ServerOptions
import vitapmate.core.LiveTopic
import vitapmate.core.ServerSession
import vitapmate.core.ServerStatus
import vitapmate.core.ServerVault
import vitapmate.core.SessionSource
import vitapmate.data.AppCore
import vitapmate.data.Gmail
import vitapmate.data.Problem
import vitapmate.data.ServerFailure
import vitapmate.data.problemOf
import vitapmate.data.rememberLive
import vitapmate.features.login.OtpAutofill
import vitapmate.features.login.ago
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppDialog
import vitapmate.ui.AppTheme
import vitapmate.ui.ConnectedChoice
import vitapmate.ui.Pips
import vitapmate.ui.StatusChip
import vitapmate.ui.heroCookie
import vitapmate.ui.neutralTone
import vitapmate.ui.ProblemView
import vitapmate.ui.ProofProgress
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast
import vitapmate.data.attempt

/** Route id of Settings → Account → Server. */
const val SERVER = "server"

private val PHONE_WAITS = listOf(10u, 20u, 45u)
private val VAULT_TTLS = listOf<Long?>(86_400, 172_800, null)

/** A destructive action waiting for its confirm dialog; [call] is false when the server lacks it. */
private class Pending(val title: String, val text: String, val confirm: String, val name: String, val done: String, val call: suspend (Core, String) -> Boolean)

/**
 * Server (Rust's `server.rs`): everything the server keeps for this student and this phone
 * (account, sealed session and password, options, phones, keys, the forwarding address, sign-ins,
 * this install's registration), each with a way to delete it (asks first, saying what goes).
 * Actions the deployed server doesn't offer yet are disabled once it says so. Flows `server.*`,
 * lines `ui.server`.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ServerScreen() {
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    val nav = LocalNav.current
    var status by remember { mutableStateOf<ServerStatus?>(null) }
    var problem by remember { mutableStateOf<Problem?>(null) }
    var loading by remember { mutableStateOf(false) }
    var saving by remember { mutableStateOf(false) }
    var pending by remember { mutableStateOf<Pending?>(null) }
    // This phone removed, or the account deleted ([Pending.name]): not loaded again on its own,
    // since loading registers the phone (and proves the account) afresh.
    var removed by remember { mutableStateOf<String?>(null) }
    // Actions the server answered 404 to (`Pending.name`).
    var unsupported by remember { mutableStateOf(setOf<String>()) }

    fun load() {
        scope.launch {
            loading = true
            removed = null
            run("load") { c, f -> c.serverStatus(f) }
                .onSuccess { status = it; problem = null }
                .onFailure { e -> if (status == null) problem = (e as ServerFailure).problem }
            loading = false
        }
    }
    val change by rememberLive(LiveTopic.SERVER, status != null)
    LaunchedEffect(change) { if (change > 0 && removed == null) load() }

    fun setOptions(options: ServerOptions) {
        scope.launch {
            saving = true
            run("options") { c, f -> c.setServerOptions(f, options) }.onSuccess {
                haptics.confirm()
                status = status?.copy(options = options)
            }
            saving = false
        }
    }

    fun ask(title: String, text: String, confirm: String, name: String, done: String, call: suspend (Core, String) -> Boolean) {
        // One action at a time: a second tap while one runs would ask the server twice.
        if (!saving) pending = Pending(title, text, confirm, name, done, call)
    }

    fun trash(tag: String, description: String, enabled: Boolean = true, onClick: () -> Unit): @Composable () -> Unit = {
        IconButton(onClick, Modifier.testTag(tag), enabled = enabled) { Icon(Lucide.Trash2, description) }
    }

    Page("Server", SERVER) { padding ->
        LazyColumn(contentPadding = padding) {
            if (status != null && removed == null) item { LiveProblemBanner(LiveTopic.SERVER, ::load) }
            val s = status
            when {
                removed != null -> item {
                    val deleted = removed == "delete"
                    ListGroup(
                        if (deleted) "Account" else "This phone",
                        footer = if (deleted) "Loading again proves who you are and starts a new, empty account." else "The server registers this phone again the next time the app talks to it, such as when this page loads.",
                    ) {
                        if (deleted) GroupItem("Deleted from the server", "server.deleted", Lucide.Trash2, "Keys, phones, session, password and history are gone")
                        else GroupItem("Removed from the server", "server.removed", Lucide.Unplug, "Its keys and push token are gone")
                        GroupItem("Load again", "server.reload", Lucide.Server, onClick = ::load)
                    }
                }
                s != null -> {
                    item { AccountHero(s) }
                    if (s.linked) item {
                        ListGroup("Kept on the server", footer = "Sessions and passwords are sealed; only the server's sign-in engine can open them.") {
                            val savePassword = {
                                val gmail = OtpAutofill.serverGrant()
                                ask(
                                    "Let the server sign in for you?",
                                    "The server keeps your VTOP password sealed, so connected apps get a session while your phone is off. It reads the sign-in code from your " +
                                        (if (gmail != null) "Gmail" else "forwarding address") + ". Only its sign-in engine can open the password, and you can forget it here any time.",
                                    "Allow",
                                    "save_password",
                                    "The server keeps your password.",
                                ) { c, f ->
                                    c.saveServerPassword(f, gmail)
                                    true
                                }
                            }
                            Row(Modifier.height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                                Tile("VTOP session", "server.session", Lucide.Cookie, CircleShape, sessionText(s), false, Modifier.weight(1f)) {
                                    FilledTonalButton(
                                        onClick = {
                                            ask("Clear the session?", "The server drops the VTOP session it keeps. Connected apps get a fresh one on their next request.", "Clear", "clear_session", "The server's session is cleared.") { c, f -> c.clearServerSession(f); true }
                                        },
                                        modifier = Modifier.padding(top = 10.dp).testTag("server.clear_session"),
                                        enabled = s.session is ServerSession.Saved,
                                        shapes = ButtonDefaults.shapes(),
                                    ) { Text("Clear") }
                                }
                                val vault = s.vault
                                val kept = vault is ServerVault.Saved
                                Tile("Password", "server.password", Lucide.Lock, MaterialShapes.Cookie7Sided.toShape(), passwordText(s), false, Modifier.weight(1f), onClick = if (kept) null else savePassword) {
                                    if (kept) {
                                        FilledTonalButton(
                                            onClick = {
                                                ask("Forget the saved password?", "The server drops your sealed VTOP password${if ((vault as? ServerVault.Saved)?.gmail == true) " and Gmail access" else ""}. It can then no longer sign in by itself: connected apps work only while a phone answers.", "Forget", "forget_password", "The server no longer keeps your password.") { c, f -> c.forgetServerPassword(f); true }
                                            },
                                            modifier = Modifier.padding(top = 10.dp).testTag("server.forget_password"),
                                            shapes = ButtonDefaults.shapes(),
                                        ) { Text("Forget") }
                                    } else {
                                        Button(onClick = savePassword, modifier = Modifier.padding(top = 10.dp).testTag("server.save_password"), shapes = ButtonDefaults.shapes()) { Text("Allow") }
                                    }
                                }
                            }
                        }
                    }
                    if (s.linked) item {
                        val o = s.options
                        ListGroup("Options", footer = "How the server gets a VTOP session when a connected app needs one.") {
                            GroupItem(
                                "Always sign in with the saved password",
                                "server.always_vault",
                                Lucide.Zap,
                                if (o.alwaysUseVault) "Without asking a phone first" else "Asks a phone first",
                                enabled = !saving,
                                onClick = { setOptions(o.copy(alwaysUseVault = !o.alwaysUseVault)) },
                                trailing = { ItemSwitch(o.alwaysUseVault, { setOptions(o.copy(alwaysUseVault = it)) }, enabled = !saving) },
                                on = o.alwaysUseVault,
                            )
                            GroupSegment(Modifier.testTag("server.option_choices")) {
                                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                    if (!o.alwaysUseVault) {
                                        Label("Wait for the phone before signing in")
                                        ConnectedChoice(PHONE_WAITS, o.phoneWaitSecs, { if (!saving) setOptions(o.copy(phoneWaitSecs = it)) }, tag = "server.phone_wait", key = { "$it" }, label = { "$it s" })
                                    }
                                    Label("Keep the saved password")
                                    ConnectedChoice(VAULT_TTLS, o.vaultTtlSecs, { if (!saving) setOptions(o.copy(vaultTtlSecs = it)) }, tag = "server.vault_ttl", key = { "${it ?: "rejected"}" }, label = { ttlText(it) })
                                }
                            }
                        }
                    }
                    if (s.phones.isNotEmpty()) item {
                        ListGroup("Linked phones", footer = "Linked phones are asked for VTOP sessions. A removed phone links again the next time it proves who you are.") {
                            s.phones.forEach { phone ->
                                GroupItem(phone.label ?: "Phone", "server.phone.${phone.id}", Lucide.Smartphone, phoneText(phone.thisPhone, phone.seenAt, phone.addedAt), below = phoneChip(phone.thisPhone, phone.seenAt, phone.addedAt), trailing = if (phone.thisPhone) null else trash("server.phone.${phone.id}.remove", "Remove") {
                                    ask("Remove ${phone.label ?: "this phone"}?", "It is unlinked from your account, no longer asked for VTOP sessions, and stops getting your forwarded OTPs. Keys it made stop getting sessions from it.", "Remove", "remove_phone", "It is no longer linked.") { c, f -> c.removePhone(f, phone.id); true }
                                })
                            }
                        }
                    }
                    item {
                        ListGroup("Access keys", footer = if (s.linked) null else "Keys show here once this phone is linked.") {
                            s.keys.forEach { key ->
                                GroupItem(key.label, "server.key.${key.id}", key.kind.icon, keyText(key), iconShape = key.kind.shape, trailing = trash("server.key.${key.id}.revoke", "Revoke") {
                                    ask("Revoke ${key.label}?", "The app using this key loses access to your VTOP account at once.", "Revoke", "revoke_key", "The key no longer works.") { c, f -> c.revokeKey(f, key.id); true }
                                })
                            }
                            GroupItem("Connected apps", "server.connected_apps", Lucide.Unplug, "Add a key for the extension or an MCP client", onClick = { nav.push(Route.Screen(CONNECTED_APPS)) })
                        }
                    }
                    item {
                        val mailbox = s.mailbox
                        ListGroup("Mail forwarding", footer = when {
                            mailbox != null -> "Gmail forwards VTOP's OTP mail here; the server seals each code to the phones on the address and keeps it 5 minutes."
                            s.forwarding -> "Your forwarding address is on another phone. Setting up OTP auto-fill here joins it."
                            else -> null
                        }) {
                            if (mailbox == null) {
                                GroupItem(if (s.forwarding) "On another phone" else "No forwarding address", "server.mail", Lucide.Mail)
                            } else {
                                GroupSegment(Modifier.testTag("server.mail")) {
                                    Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                                        Box(Modifier.size(40.dp).background(MaterialTheme.colorScheme.secondaryContainer, CircleShape), Alignment.Center) {
                                            Icon(Lucide.Mail, null, Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSecondaryContainer)
                                        }
                                        Column(Modifier.weight(1f)) {
                                            Text(mailbox.address, style = MaterialTheme.typography.bodyLarge.copy(fontFamily = FontFamily.Monospace), color = MaterialTheme.colorScheme.onSurface)
                                            Text("${mailbox.phones.size} of 5 phones", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                                        }
                                    }
                                    Pips(mailbox.phones.size, 5, Modifier.padding(top = 12.dp))
                                }
                                mailbox.phones.forEach { phone ->
                                    GroupItem(phone.label ?: "Phone", "server.mail.${phone.id.take(8)}", Lucide.Smartphone, phoneText(phone.thisPhone, phone.seenAt, phone.addedAt), below = phoneChip(phone.thisPhone, phone.seenAt, phone.addedAt), trailing = if (phone.thisPhone) null else trash("server.mail.${phone.id.take(8)}.remove", "Remove") {
                                        ask("Take ${phone.label ?: "this phone"} off the address?", "It stops getting forwarded OTPs. The address keeps working on the other phones.", "Remove", "remove_mail_phone", "It no longer gets forwarded OTPs.") { c, f -> c.removeForwardingPhone(f, phone.id); true }
                                    })
                                }
                                GroupItem("Leave the address", "server.mail.leave", Lucide.LogOut, "This phone stops getting forwarded OTPs", onClick = {
                                    ask("Leave the address?", "This phone stops getting forwarded OTPs.${if (mailbox.phones.size <= 1) " It's the only phone on it, so the address is deleted and Gmail's forwarding to it stops working." else " The other phones keep it."}", "Leave", "leave_mail", "This phone left the address.") { c, f -> c.turnOffForwarding(f); true }
                                })
                                GroupItem("Delete the address everywhere", "server.mail.delete", Lucide.Trash2, "For every phone on it", tone = AppTheme.tones.danger, onClick = {
                                    ask("Delete the address everywhere?", "The address is deleted for all ${mailbox.phones.size} phones; mail Gmail forwards to it is rejected. Remove the forwarding in Gmail too.", "Delete", "delete_mail", "The forwarding address is deleted.") { c, f -> c.deleteForwardingEverywhere(f); true }
                                })
                            }
                        }
                    }
                    if (s.linked && s.signIns.isNotEmpty()) item {
                        ListGroup("Last sign-ins") {
                            s.signIns.forEachIndexed { i, signIn ->
                                TimelineItem(
                                    "${if (signIn.ok) "Signed in" else "Failed"} on ${if (signIn.byServer) "the server" else "a phone"}",
                                    "server.sign_in.$i",
                                    ago(signIn.at),
                                    signIn.reason,
                                    failed = !signIn.ok,
                                )
                            }
                            val off = "clear_sign_ins" in unsupported
                            GroupItem("Clear history", "server.clear_sign_ins", Lucide.History, if (off) "Needs a server update" else "Forget these ${s.signIns.size} sign-ins", enabled = !off, onClick = {
                                ask("Clear the sign-in history?", "The server forgets these ${s.signIns.size} sign-ins. New ones are recorded again.", "Clear", "clear_sign_ins", "The sign-in history is cleared.") { c, f -> c.clearServerSignIns(f) }
                            })
                        }
                    }
                    item {
                        val d = s.device
                        val off = "remove_device" in unsupported
                        ListGroup("This phone on the server", footer = "Requests carry a short-lived token from your account proof, renewed with a key only this phone holds.") {
                            GroupItem(d.label, "server.device", Lucide.Smartphone, listOfNotNull(
                                d.id?.let { "Id $it…" } ?: "Not registered",
                                d.registeredAt?.let { "registered ${ago(it)}" },
                            ).joinToString(" · "))
                            GroupItem("Push notifications", "server.device.push", Lucide.Server, below = {
                                Row(Modifier.padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                                    if (d.pushToken) StatusChip("Token set", AppTheme.tones.success) else StatusChip("No token", neutralTone)
                                }
                            })
                            GroupItem("Remove this phone from the server", "server.device.remove", Lucide.Unplug, if (off) "Needs a server update" else "Its keys and push token", enabled = d.id != null && !off, tone = AppTheme.tones.danger, onClick = {
                                ask(
                                    "Remove this phone from the server?",
                                    "The server deletes this phone's keys and push token${if (s.linked) ", unlinks it from your account" else ""}${if (s.mailbox != null) " and takes it off the forwarding address (deleting the address if it's the only phone on it)" else ""}. Your account and its keys stay. The phone registers again the next time it talks to the server.",
                                    "Remove",
                                    "remove_device",
                                    "This phone is removed from the server.",
                                ) { c, f -> c.deleteServerDevice(f) }
                            })
                        }
                    }
                    if (s.linked) item {
                        val danger = AppTheme.tones.danger
                        Column(Modifier.fillMaxWidth().padding(start = 18.dp, top = 14.dp, end = 18.dp, bottom = 12.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                            Button({
                                ask(
                                    "Delete everything on the server?",
                                    "The server deletes your account: every access key stops working, every phone is unlinked, and the saved session, password, options, sign-in history and forwarding address go. Gmail's forwarding to it stops working; set it up again to get OTPs filled in. This phone's registration and your data on this phone stay.",
                                    "Delete",
                                    "delete",
                                    "The server keeps no account for you now.",
                                ) { c, f -> c.deleteServerAccount(f); true }
                            }, Modifier.fillMaxWidth().height(58.dp).testTag("server.delete"), shape = CircleShape, colors = ButtonDefaults.buttonColors(containerColor = danger.subtle, contentColor = danger.onSubtle)) {
                                Icon(Lucide.Trash2, null, Modifier.size(20.dp), tint = danger.base)
                                Spacer(Modifier.width(10.dp))
                                Text("Delete everything", style = MaterialTheme.typography.titleSmallEmphasized)
                            }
                            Text(
                                "Your account, keys, phones, session, password and history",
                                Modifier.padding(top = 8.dp),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                textAlign = TextAlign.Center,
                            )
                        }
                    }
                }
                problem != null -> item { problem?.let { ProblemView(it, onRetry = ::load, Modifier.fillMaxWidth(), busy = loading) } }
                else -> item { ProofProgress(Modifier.testTag("server.loading")) }
            }
        }
    }

    pending?.let { p ->
        AppDialog(
            { pending = null },
            p.title,
            p.confirm,
            {
                pending = null
                scope.launch {
                    saving = true
                    run(p.name, p.call).onSuccess { done ->
                        if (!done) {
                            unsupported = unsupported + p.name
                            showToast("Server", "The server doesn't offer this yet.")
                            return@onSuccess
                        }
                        haptics.confirm()
                        showToast("Server", p.done)
                        if (p.name == "remove_device" || p.name == "delete") { removed = p.name; status = null } else load()
                    }.onFailure { haptics.reject() }
                    saving = false
                }
            },
            icon = Lucide.TriangleAlert,
            danger = true,
            confirmTag = "server.confirm",
        ) { Text(p.text) }
    }
}

@Composable
private fun Label(text: String) = Text(text, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)

private fun ttlText(secs: Long?) = when (secs) {
    null -> "Until rejected"
    86_400L -> "1 day"
    else -> "${secs / 86_400} days"
}

private fun sessionText(s: ServerStatus) = when (val session = s.session) {
    ServerSession.None -> "None kept"
    is ServerSession.Saved -> "Saved ${ago(session.at)} by ${when (session.by) {
        SessionSource.SERVER -> "the server"
        SessionSource.PHONE -> "a phone"
    }}"
}

private fun passwordText(s: ServerStatus): String = when (val vault = s.vault) {
    ServerVault.None -> "Not kept. Tap to let the server sign in for you"
    is ServerVault.Saved -> {
        val expires = vault.expiresAt?.let { "expires in ${((it - System.currentTimeMillis() / 1000) / 3600).coerceAtLeast(0)} h" } ?: "kept until VTOP rejects it"
        listOfNotNull("Saved ${ago(vault.at)}", "with Gmail access".takeIf { vault.gmail }, expires).joinToString(" · ")
    }
}

/** A phone's dates; null for this phone, whose row shows them after a chip ([phoneChip]). */
private fun phoneText(thisPhone: Boolean, seenAt: Long, addedAt: Long) =
    if (thisPhone) null else "seen ${ago(seenAt)} · added ${ago(addedAt)}"

/** "This phone" as an accent chip, then the dates; null for other phones. */
private fun phoneChip(thisPhone: Boolean, seenAt: Long, addedAt: Long): (@Composable () -> Unit)? = if (!thisPhone) null else {
    {
        Row(Modifier.padding(top = 4.dp), horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
            StatusChip("This phone", AppTheme.tones.accentTone, solid = true)
            Text("seen ${ago(seenAt)} · added ${ago(addedAt)}", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

/** The account as a hero (Account's header style): who, since when, linked or not, its semester and a rejected password. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun AccountHero(s: ServerStatus) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).clip(MaterialTheme.shapes.extraLarge)
            .background(tones.accentTone.subtle).heroCookie().padding(18.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Row(Modifier.testTag("server.account"), horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(56.dp).background(AppTheme.colors.card, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                Icon(Lucide.User, null, Modifier.size(24.dp), tint = scheme.onSurface)
            }
            Column(Modifier.weight(1f)) {
                Text(s.registrationNumber, style = MaterialTheme.typography.titleLargeEmphasized, color = scheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
                val host = rememberServer().host
                Text(s.createdAt?.let { "On $host since ${ago(it)}" } ?: host, style = MaterialTheme.typography.bodyMedium, color = tones.accentTone.onSubtle)
            }
        }
        if (s.linked) StatusChip("Linked", tones.success) else StatusChip("Not linked", neutralTone)
        s.credentialsRejectedAt?.let {
            Row(Modifier.testTag("server.rejected"), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                StatusChip("VTOP rejected the saved password", tones.warning, Modifier.weight(1f, fill = false), Lucide.TriangleAlert)
                Text(ago(it), style = MaterialTheme.typography.bodyMedium, color = tones.accentTone.onSubtle)
            }
        }
        if (s.linked) {
            Row(
                Modifier.fillMaxWidth().background(AppTheme.colors.card, MaterialTheme.shapes.large).padding(horizontal = 12.dp, vertical = 10.dp).testTag("server.semester"),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(Lucide.CalendarRange, null, Modifier.size(18.dp), tint = scheme.onSurfaceVariant)
                Text("Semester for connected apps", Modifier.weight(1f), style = MaterialTheme.typography.bodyMedium, color = scheme.onSurface)
                Text(s.semesterId ?: "The newest", style = MaterialTheme.typography.labelLarge, color = scheme.onSurface)
            }
        } else {
            Text(
                "This phone isn't linked to your server account, so the server shows it only what applies to this phone. It links again the next time it proves who you are to the server.",
                style = MaterialTheme.typography.bodySmall,
                color = tones.accentTone.onSubtle,
            )
        }
    }
}

private fun keyText(key: AccessKey) =
    listOfNotNull("Made ${ago(key.createdAt)}", key.lastUsedAt?.let { "last used ${ago(it)}" } ?: "never used").joinToString(" · ")

/** One call as flow `server.<name>`, off the main thread; a failure is logged, toasted and returned as a [ServerFailure]. */
private suspend fun <T> run(name: String, call: suspend (Core, String) -> T): Result<T> {
    var flowId = ""
    return attempt {
        Result.success(flow("server.$name") { f -> flowId = f; withContext(Dispatchers.IO) { call(AppCore.await(), f) } })
    }.getOrElse { e ->
        Log.warn("ui.server", "$name failed: ${e::class.simpleName}", flowId)
        when (e) {
            // Rust's prompt asked for the code and the student closed it: nothing to add.
            is CoreException.OtpRequired -> {}
            is CoreException.Other -> showToast("Server", e.reason)
            else -> showToast(problemOf(e, flowId, server = true))
        }
        Result.failure(ServerFailure(flowId, e))
    }
}
