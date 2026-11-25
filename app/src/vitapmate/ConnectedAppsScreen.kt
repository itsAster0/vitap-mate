package vitapmate

import android.Manifest
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.toShape
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.BellOff
import com.composables.icons.lucide.Copy
import com.composables.icons.lucide.Globe
import com.composables.icons.lucide.KeyRound
import com.composables.icons.lucide.Download
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Link
import com.composables.icons.lucide.Smartphone
import vitapmate.core.KeyOwner
import vitapmate.core.KeyPhone
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Scan
import com.composables.icons.lucide.Sparkles
import kotlinx.coroutines.launch
import vitapmate.core.AccessKey
import vitapmate.core.ConnectedApps
import vitapmate.core.Core
import vitapmate.core.KeyKind
import vitapmate.core.LiveTopic
import vitapmate.core.ServerPush
import vitapmate.core.defaultServer
import vitapmate.data.Foreground
import vitapmate.data.Settings
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.data.rememberLive
import vitapmate.observability.Log
import vitapmate.features.login.ago
import vitapmate.ui.AppDialog
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.Pips
import vitapmate.ui.ProblemView
import vitapmate.ui.ProofProgress
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast

/** Route id of Settings → Account → Connected apps. */
const val CONNECTED_APPS = "connected_apps"

/** The extension's latest build (Chrome zip and signed Firefox add-on), replaced by CI. */
private const val EXTENSION_URL = "https://github.com/itsAster0/vitap-mate/releases/tag/extension-latest"

/**
 * The server this app talks to: the developer override (Settings → Developer), else Rust's
 * default, so its address and MCP link follow it.
 */
@Composable
internal fun rememberServer(): String = produceState(defaultServer()) {
    value = orNull { coreFlow("ui.server_url") { c, f -> c.devForwardingServer(f) } }?.ifBlank { null } ?: value
}.value

internal val String.host get() = substringAfter("://").substringBefore('/')

/**
 * Connected apps (Rust's `keys.rs`): the access keys of the browser extension and AI apps (MCP),
 * each with its last uses, outings switch and revoke; "Pair a browser" (scan the extension's or
 * the AI app's QR code or type its code), "Copy MCP link" and "Make a key by hand" (shown once).
 * Making a key asks for the screen lock and links this phone to the server account the first time.
 * Below the keys: "Session handover" ([SessionHandover]: the cookie question and what the server
 * stores, off by default) and the semester and recent sign-ins. Flows `connected_apps.*`, lines
 * `ui.connected_apps`; keys, codes and cookies are never logged.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ConnectedAppsScreen() {
    val scope = rememberCoroutineScope()
    val uri = LocalUriHandler.current
    val context = LocalContext.current
    val mcpUrl = rememberServer() + "/mcp"
    var phase by remember { mutableStateOf<ServerLoad<ConnectedApps>>(ServerLoad.Loading) }
    var pairing by remember { mutableStateOf(false) }
    var newMcp by remember { mutableStateOf(false) }
    var signIns by remember { mutableStateOf(false) }
    var opened by remember { mutableStateOf<AccessKey?>(null) }
    val deviceAuth = rememberDeviceAuth()
    val haptics = rememberHaptics()
    var busy by remember { mutableStateOf(false) }
    var moving by remember { mutableStateOf<KeyPhone?>(null) }
    val actions = rememberServerActions(appsCall)

    fun load() {
        scope.launch { phase = phase.after(appsCall("load") { c, f -> c.connectedApps(f) }) }
    }
    val change by rememberLive(LiveTopic.ACCOUNT, phase is ServerLoad.Ready)
    LaunchedEffect(change) { if (change > 0) load() }
    /** A change to the account's phones behind the screen lock ([title]), then the keys again. */
    fun change(title: String, name: String, done: String, call: suspend (Core, String) -> Unit) {
        deviceAuth.confirm(title) {
            scope.launch {
                busy = true
                appsCall(name, call).onSuccess { haptics.confirm(); showToast(done, ""); load() }.onFailure { haptics.reject() }
                busy = false
            }
        }
    }

    Page("Connected apps", CONNECTED_APPS) { padding ->
        LazyColumn(contentPadding = padding) {
            if (phase is ServerLoad.Ready) item { LiveProblemBanner(LiveTopic.ACCOUNT, ::load) }
            item {
                ListGroup("Add", footer = "The browser extension signs you in to VTOP in Chrome or Firefox; AI apps such as Claude read your VTOP data over MCP. Neither sees your password.") {
                    Row(Modifier.height(IntrinsicSize.Min), horizontalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                        Tile("Pair a browser", "connected_apps.pair", Lucide.Scan, MaterialShapes.Cookie9Sided.toShape(), "Scan the code the extension or an AI app's sign-in page shows", true, Modifier.weight(1f), onClick = { pairing = true })
                        Tile("Copy MCP link", "connected_apps.copy_mcp", Lucide.Copy, MaterialShapes.Sunny.toShape(), "Add it to Claude or another AI app, then pair the code it shows", false, Modifier.weight(1f), onClick = { copy(context, "MCP link", mcpUrl) })
                    }
                    GroupItem("Make a key by hand", "connected_apps.new_mcp", Lucide.KeyRound, "For AI apps that take a key instead of signing in", onClick = { newMcp = true }, trailing = { Chevron() })
                    GroupItem("Get the extension", "connected_apps.get_extension", Lucide.Download, "Chrome zip and Firefox add-on, from GitHub", onClick = { uri.openUri(EXTENSION_URL) }, trailing = { Icon(Lucide.ExternalLink, null, tint = MaterialTheme.colorScheme.onSurfaceVariant) })
                }
            }
            when (val p = phase) {
                is ServerLoad.Ready -> item {
                    val current = p.value
                    ListGroup("Keys", footer = "Up to 10 keys. Revoke one you no longer use; it stops working at once.", titleEnd = if (current.linked) "${current.keys.size} of 10" else null, belowTitle = if (current.linked) ({ Pips(current.keys.size, 10) }) else null) {
                        // After a reinstall: the server lists the account's keys only to a linked phone.
                        if (!current.linked) GroupItem(
                            "Link this phone",
                            "connected_apps.link",
                            Lucide.Link,
                            "Your keys show once this phone is linked to your account",
                            enabled = !busy,
                            onClick = { change("Link this phone", "link", "Phone linked") { c, f -> c.linkPhone(f) } },
                            trailing = { Chevron() },
                        ) else if (current.keys.isEmpty()) GroupItem("No keys yet", "connected_apps.empty", Lucide.KeyRound, "Pair a browser or an AI app above")
                        if (current.keys.isNotEmpty()) PushWarning(current.push)
                        current.keys.forEach { key ->
                            GroupItem(key.label, "connected_apps.key.${key.id}", key.kind.icon, onClick = { opened = key }, trailing = { Chevron() }, iconShape = key.kind.shape, below = { KeyTags(key) })
                        }
                    }
                    if (current.elsewhere.isNotEmpty()) ListGroup("On another phone", footer = "A key asks the phone that made it for your VTOP session. After a reinstall, move its keys here.") {
                        current.elsewhere.forEachIndexed { i, phone ->
                            val accent = AppTheme.tones.accentTone
                            Column(Modifier.fillMaxWidth().clip(ListItemDefaults.segmentedShapes(1, 3).shape).background(accent.subtle).padding(16.dp)) {
                                Row(horizontalArrangement = Arrangement.spacedBy(12.dp), verticalAlignment = Alignment.CenterVertically) {
                                    Box(Modifier.size(40.dp).background(AppTheme.colors.card, CircleShape), Alignment.Center) {
                                        Icon(Lucide.Smartphone, null, Modifier.size(20.dp), tint = MaterialTheme.colorScheme.onSurface)
                                    }
                                    Column(Modifier.weight(1f)) {
                                        Text("Move ${keys(phone)} to this phone", style = MaterialTheme.typography.titleMediumEmphasized, color = MaterialTheme.colorScheme.onSurface)
                                        Text(
                                            when (val owner = phone.owner) {
                                                is KeyOwner.Phone -> "From ${owner.label ?: "another phone"}, seen ${ago(owner.seenAt)}"
                                                KeyOwner.Unlinked -> "From a phone that is no longer linked"
                                            },
                                            style = MaterialTheme.typography.bodyMedium,
                                            color = accent.onSubtle,
                                        )
                                    }
                                }
                                androidx.compose.material3.Button(onClick = { moving = phone }, modifier = Modifier.fillMaxWidth().padding(top = 14.dp).height(48.dp).testTag("connected_apps.move.$i"), enabled = !busy, shapes = ButtonDefaults.shapes()) { Text("Move") }
                            }
                        }
                    }
                }
                is ServerLoad.Failed -> item { ProblemView(p.problem, onRetry = { phase = ServerLoad.Loading; load() }, Modifier.fillMaxWidth()) }
                ServerLoad.Loading -> item { ProofProgress(Modifier.testTag("connected_apps.loading")) }
            }
            phase.ready?.let { current ->
                item { SessionHandover(current, actions, ::load) }
                item { HandoverDetails(current) { signIns = true } }
            }
        }
    }

    moving?.let { phone ->
        AppDialog(
            { moving = null },
            "Move ${keys(phone)} to this phone?",
            "Move",
            {
                moving = null
                change("Move keys", "move", "Keys moved") { c, f -> c.moveKeys(f, (phone.owner as? KeyOwner.Phone)?.id) }
            },
            icon = Lucide.Smartphone,
            confirmTag = "connected_apps.move.confirm",
        ) {
            val unlinks = when (val owner = phone.owner) {
                is KeyOwner.Phone -> " ${owner.label ?: "The other phone"} is unlinked from your account; it can link again."
                KeyOwner.Unlinked -> ""
            }
            Text("This phone will answer for ${if (phone.keys == 1u) "it" else "them"}.$unlinks")
        }
    }
    actions.Dialog { done ->
        // Choosing storage here answers the one-time note about it.
        if (done.name.startsWith("storage")) Settings.storageNoteSeen = true
        load()
    }
    if (signIns) SignInsSheet { signIns = false }
    if (pairing) PairSheet(onDone = { pairing = false; load() }, onDismiss = { pairing = false })
    if (newMcp) NewMcpSheet(onDone = { load() }, onDismiss = { newMcp = false })
    opened?.let { key -> KeySheet(key, onChanged = { opened = null; load() }, onDismiss = { opened = null }) }
}

/**
 * Keys that can't reach this phone while the app is closed: notifications not allowed (tap asks)
 * or no push token on the server ([ServerPush.NO_TOKEN]). Read again on each return to the app.
 */
@Composable
private fun PushWarning(push: ServerPush) {
    val context = LocalContext.current
    val resumes = Foreground.resumes
    val canNotify = remember(resumes) { Push.canNotify(context) }
    val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        Log.info("ui.connected_apps", "notifications", fields = mapOf("granted" to "$granted"))
    }
    when {
        !canNotify -> GroupItem(
            "Allow notifications",
            "connected_apps.push_warning",
            Lucide.BellOff,
            "Your apps' session requests reach this phone only as a notification while the app is closed",
            onClick = { ask.launch(Manifest.permission.POST_NOTIFICATIONS) },
            tone = AppTheme.tones.warning,
        )
        push == ServerPush.NO_TOKEN -> GroupItem(
            "The server can't wake this phone",
            "connected_apps.push_warning",
            Lucide.BellOff,
            "It has no push token for it (Google Play services may be missing). Keep the app open when an app needs your session.",
            tone = AppTheme.tones.warning,
        )
    }
}

private fun keys(phone: KeyPhone) = if (phone.keys == 1u) "1 key" else "${phone.keys} keys"

internal val KeyKind.icon get() = if (this == KeyKind.BROWSER) Lucide.Globe else Lucide.Sparkles
internal val KeyKind.label get() = if (this == KeyKind.BROWSER) "Browser" else "MCP"

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
internal val KeyKind.shape: Shape @Composable get() = if (this == KeyKind.BROWSER) CircleShape else MaterialShapes.Cookie7Sided.toShape()

/** A key's kind, last use and (in the warning tone) outings, as tags under its label. */
@Composable
private fun KeyTags(key: AccessKey) {
    val scheme = MaterialTheme.colorScheme
    val tag = @Composable { text: String, fill: Color, color: Color ->
        Text(text, Modifier.background(fill, CircleShape).padding(horizontal = 8.dp, vertical = 2.dp), style = MaterialTheme.typography.labelMedium, color = color)
    }
    Row(Modifier.padding(top = 5.dp), horizontalArrangement = Arrangement.spacedBy(5.dp), verticalAlignment = Alignment.CenterVertically) {
        tag(key.kind.label, scheme.surfaceContainerHigh, scheme.onSurface)
        Text(key.lastUsedAt?.let { "used ${ago(it)}" } ?: "never used", style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        if (key.outings) tag("outings on", AppTheme.tones.warning.subtle, AppTheme.tones.warning.base)
    }
}

/** Connected apps' calls, as flows `connected_apps.<name>`. */
internal val appsCall = ServerCalls("connected_apps", "Connected apps")


internal fun copy(context: Context, label: String, text: String) {
    context.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText(label, text))
    showToast("Copied", label)
}
