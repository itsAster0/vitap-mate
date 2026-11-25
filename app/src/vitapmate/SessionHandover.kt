package vitapmate

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarRange
import com.composables.icons.lucide.Cookie
import com.composables.icons.lucide.Database
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Info
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Zap
import vitapmate.core.ConnectedApps
import vitapmate.core.CookieKind
import vitapmate.core.ServerSession
import vitapmate.core.ServerStorage
import vitapmate.core.SessionSource
import vitapmate.core.VaultOptions
import vitapmate.data.Settings
import vitapmate.features.login.OtpAutofill
import vitapmate.features.login.ago
import vitapmate.ui.ConnectedChoice

private val PHONE_WAITS = listOf(10u, 20u, 45u)
private val VAULT_TTLS = listOf<Long?>(86_400, 172_800, null)

/**
 * Connected apps' "Session handover" (Rust's [ServerStorage]): asking before the cookie goes,
 * and what the server stores for offline use (off by default): the cached session with Clear,
 * then a saved password and how the server uses it. Each change goes through [actions] (the
 * screen reads again after it); [reload] after an option.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun SessionHandover(apps: ConnectedApps, actions: ServerActions, reload: () -> Unit) {
    val storage = apps.storage
    val on = storage !is ServerStorage.Off
    val session = when (storage) {
        ServerStorage.Off -> ServerSession.None
        is ServerStorage.SessionOnly -> storage.session
        is ServerStorage.OfflineSignIn -> storage.session
    }
    val vault = storage as? ServerStorage.OfflineSignIn
    val setStorage = { enabled: Boolean ->
        actions.ask(
            if (enabled) {
                ServerConfirm(
                    "Store your session on the server?",
                    "When this phone hands a VTOP session to a connected app, the server keeps it sealed for up to a week, so the app keeps working while your phone is off or offline. Only the server's sign-in engine can open it. Turn this off any time to delete it.",
                    "Turn on",
                    "storage_on",
                    "The server keeps sessions for your connected apps.",
                    danger = false,
                ) { c, f -> c.setServerStorage(f, true); true }
            } else {
                ServerConfirm(
                    "Stop storing on the server?",
                    "The server deletes the session it keeps${if (vault != null) " and your saved password" else ""}. Connected apps then get a session only while this phone answers.",
                    "Turn off",
                    "storage_off",
                    "The server keeps nothing of your VTOP now.",
                ) { c, f -> c.setServerStorage(f, false); true }
            },
        )
    }
    val setPassword = { save: Boolean ->
        actions.ask(
            if (save) {
                val gmail = OtpAutofill.serverGrant()
                ServerConfirm(
                    "Save your password on the server?",
                    "The server keeps your VTOP password sealed, so connected apps get a session while your phone is off. It reads the sign-in code from your " +
                        (if (gmail != null) "Gmail" else "forwarding address") + ". Only its sign-in engine can open the password, and you can forget it here any time.",
                    "Save",
                    "save_password",
                    "The server keeps your password.",
                    danger = false,
                ) { c, f -> c.saveServerPassword(f, gmail); true }
            } else {
                ServerConfirm(
                    "Forget the saved password?",
                    "The server drops your sealed VTOP password${if (vault?.vault?.gmail == true) " and Gmail access" else ""}. It can then no longer sign in by itself: connected apps work only while a phone answers or the stored session lasts.",
                    "Forget",
                    "forget_password",
                    "The server no longer keeps your password.",
                ) { c, f -> c.forgetServerPassword(f); true }
            },
        )
    }

    ListGroup(
        "Session handover",
        footer = "A connected app that needs your VTOP session asks this phone. Stored sessions and passwords are sealed; only the server's sign-in engine can open them.",
    ) {
        CookieAskSetting(CookieKind.HANDOVER, "Ask before handing over your VTOP cookie", "connected_apps.ask_cookie")
        GroupItem(
            "Store on server for offline use",
            "connected_apps.storage",
            Lucide.Database,
            if (on) "Connected apps work while your phone is off" else "Off: nothing of your VTOP is kept",
            enabled = !actions.busy,
            onClick = { setStorage(!on) },
            trailing = { ItemSwitch(on, { setStorage(it) }, enabled = !actions.busy) },
            on = on,
        )
        // From before storage was a choice: a saved password kept it on.
        if (vault != null && !Settings.storageNoteSeen) GroupItem(
            "Storage stayed on",
            "connected_apps.storage_note",
            Lucide.Info,
            "You saved your password on the server before, so it keeps storing for you. Turn storage off to delete both.",
            trailing = { TextButton({ Settings.storageNoteSeen = true }, Modifier.testTag("connected_apps.storage_note.ok")) { Text("Got it") } },
        )
        if (on) {
            GroupItem(
                "Stored session",
                "connected_apps.session",
                Lucide.Cookie,
                sessionText(session),
                trailing = {
                    TextButton(
                        {
                            actions.ask(
                                ServerConfirm(
                                    "Clear the stored session?",
                                    "The server drops the VTOP session it keeps. Connected apps get a fresh one on their next request.",
                                    "Clear",
                                    "clear_session",
                                    "The stored session is cleared.",
                                ) { c, f -> c.clearServerSession(f); true },
                            )
                        },
                        Modifier.testTag("connected_apps.session.clear"),
                        enabled = session is ServerSession.Saved && !actions.busy,
                    ) { Text("Clear") }
                },
            )
            GroupItem(
                "Save password for offline sign-in",
                "connected_apps.password",
                Lucide.Lock,
                vault?.let { passwordText(it) } ?: "The server signs in by itself when no phone answers",
                enabled = !actions.busy,
                onClick = { setPassword(vault == null) },
                trailing = { ItemSwitch(vault != null, { setPassword(it) }, enabled = !actions.busy) },
                on = vault != null,
            )
        }
        if (vault != null) {
            val o = vault.options
            val set = { options: VaultOptions -> actions.run("options", null, { c, f -> c.setVaultOptions(f, options); true }, reload) }
            GroupItem(
                "Always sign in with the saved password",
                "connected_apps.always_vault",
                Lucide.Zap,
                if (o.alwaysUseVault) "Without asking a phone first" else "Asks a phone first",
                enabled = !actions.busy,
                onClick = { set(o.copy(alwaysUseVault = !o.alwaysUseVault)) },
                trailing = { ItemSwitch(o.alwaysUseVault, { set(o.copy(alwaysUseVault = it)) }, enabled = !actions.busy) },
                on = o.alwaysUseVault,
            )
            GroupSegment(Modifier.testTag("connected_apps.vault_choices")) {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    if (!o.alwaysUseVault) {
                        Label("Wait for the phone before signing in")
                        ConnectedChoice(PHONE_WAITS, o.phoneWaitSecs, { if (!actions.busy) set(o.copy(phoneWaitSecs = it)) }, tag = "connected_apps.phone_wait", key = { "$it" }, label = { "$it s" })
                    }
                    Label("Keep the saved password")
                    ConnectedChoice(VAULT_TTLS, o.vaultTtlSecs, { if (!actions.busy) set(o.copy(vaultTtlSecs = it)) }, tag = "connected_apps.vault_ttl", key = { "${it ?: "rejected"}" }, label = { ttlText(it) })
                }
            }
        }
    }
}

/** The semester connected apps read, and the recent sign-ins ([onSignIns] opens them). */
@Composable
internal fun HandoverDetails(apps: ConnectedApps, onSignIns: () -> Unit) {
    ListGroup("Details") {
        GroupItem(
            "Semester for connected apps",
            "connected_apps.semester",
            Lucide.CalendarRange,
            "Follows the semester picked in the app",
            trailing = { Text(apps.semesterId ?: "The newest", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurface) },
        )
        GroupItem("Recent sign-ins", "connected_apps.sign_ins", Lucide.History, "Phones and the server signing in to VTOP", onClick = onSignIns, trailing = { Chevron() })
    }
}

@Composable
private fun Label(text: String) = Text(text, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)

private fun ttlText(secs: Long?) = when (secs) {
    null -> "Until rejected"
    86_400L -> "1 day"
    else -> "${secs / 86_400} days"
}

private fun sessionText(session: ServerSession) = when (session) {
    ServerSession.None -> "None kept yet"
    is ServerSession.Saved -> "Saved ${ago(session.at)} by ${when (session.by) {
        SessionSource.SERVER -> "the server"
        SessionSource.PHONE -> "a phone"
    }}"
}

private fun passwordText(storage: ServerStorage.OfflineSignIn): String {
    val vault = storage.vault
    val expires = vault.expiresAt?.let { "expires in ${((it - System.currentTimeMillis() / 1000) / 3600).coerceAtLeast(0)} h" } ?: "kept until VTOP rejects it"
    return listOfNotNull("Saved ${ago(vault.at)}", "with Gmail access".takeIf { vault.gmail }, expires).joinToString(" · ")
}
