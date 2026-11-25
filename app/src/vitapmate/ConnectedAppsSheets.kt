package vitapmate

import android.Manifest
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.ButtonDefaults
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.input.InputTransformation
import androidx.compose.foundation.text.input.TextFieldLineLimits
import androidx.compose.foundation.text.input.maxLengthReject
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import com.composables.icons.lucide.Copy
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Scan
import com.composables.icons.lucide.Trash2
import com.composables.icons.lucide.TriangleAlert
import kotlinx.coroutines.launch
import vitapmate.core.AccessKey
import vitapmate.core.KeyKind
import vitapmate.core.LiveTopic
import vitapmate.core.NewKey
import vitapmate.core.ServerSignIn
import vitapmate.core.SignInOutcome
import vitapmate.core.pairCode
import vitapmate.data.rememberLive
import vitapmate.features.login.ago
import vitapmate.observability.Log
import vitapmate.ui.AppDialog
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonVariant
import vitapmate.ui.OpenHaptic
import vitapmate.ui.ProblemView
import vitapmate.ui.Space
import vitapmate.ui.proofStage
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast

/** `ABCDE12345` as the extension shows it. */
private fun shown(code: String) = "${code.take(5)}-${code.drop(5)}"

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun Sheet(tag: String, onDismiss: () -> Unit, full: Boolean = false, content: @Composable () -> Unit) {
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    OpenHaptic()
    ModalBottomSheet(onDismiss, Modifier.testTag(tag), sheet, containerColor = MaterialTheme.colorScheme.surfaceContainerLow) {
        Column(
            // [full]: a list that can come near the screen's height; see SemesterSheet for why it then fills it.
            Modifier.fillMaxWidth().then(if (full) Modifier.fillMaxHeight() else Modifier).verticalScroll(rememberScrollState())
                .padding(horizontal = Space.xl).padding(bottom = Space.xl),
            verticalArrangement = Arrangement.spacedBy(Space.lg),
        ) { content() }
    }
}

@Composable
private fun Note(text: String) = Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)

/**
 * Pair a browser: Scan QR (Play services' scanner: the code, or a link ending in `#code`) or type
 * the code; then the screen lock, then the key goes to the extension through the edge. An AI app's
 * OAuth sign-in page (`/authorize`) shows a code too: it is named first (`pairApp`), and its key
 * takes the app's name.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun PairSheet(onDone: () -> Unit, onDismiss: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val deviceAuth = rememberDeviceAuth()
    val haptics = rememberHaptics()
    val typing = remember { FocusRequester() }
    // A paired app's session requests reach a phone off screen only as a notification.
    val notify = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        Log.info("ui.connected_apps", "notifications", fields = mapOf("granted" to "$granted"))
    }
    var text by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    val code = pairCode(text)

    fun pair(code: String) {
        scope.launch {
            busy = true
            val app = appsCall("pair_app") { c, f -> c.pairApp(f, code) }.getOrElse { busy = false; haptics.reject(); return@launch }
            busy = false
            deviceAuth.confirm(app?.let { "Connect ${it.name} (${it.host})" } ?: "Pair a browser") {
                scope.launch {
                    busy = true
                    appsCall("pair") { c, f -> c.createKey(f, false, code) }
                        .onSuccess {
                            haptics.confirm()
                            if (app == null) showToast("Browser paired", "The extension signs in to VTOP by itself now.")
                            else showToast("${app.name} connected", "It can read your VTOP data now; outings stay off unless you turn them on.")
                            if (!Push.canNotify(context)) notify.launch(Manifest.permission.POST_NOTIFICATIONS)
                            onDone()
                        }
                        .onFailure { haptics.reject() }
                    busy = false
                }
            }
        }
    }

    val scan = rememberQrScanner(
        onResult = { raw ->
            val scanned = pairCode(raw)
            Log.info("ui.connected_apps", "scanned", fields = mapOf("pair_code" to "${scanned != null}"))
            if (scanned == null) {
                showToast("Not a pairing code", "Scan the QR code the extension or the AI app's sign-in page shows.")
            } else {
                text = shown(scanned)
                pair(scanned)
            }
        },
        onUnavailable = { why ->
            Log.info("ui.connected_apps", "scanner unavailable", fields = mapOf("why" to why))
            showToast("Scanner not ready", "Type the code the extension shows instead.")
            runCatching { typing.requestFocus() }
        },
    )

    Sheet("connected_apps.pair_sheet", onDismiss) {
        Text("Pair a browser", style = MaterialTheme.typography.titleLarge)
        Note("Open the vitapmate extension in Chrome or Firefox, or connect an AI app such as Claude to ${rememberServer()}/mcp. It shows a QR code and a 10-character code.")
        Button(if (busy) null else scan, Modifier.fillMaxWidth().height(60.dp).testTag("connected_apps.scan"), prefix = Lucide.Scan) { Text("Scan QR code") }
        OutlinedTextField(
            vitapmate.ui.rememberSyncedTextFieldState(text) { text = it },
            Modifier.fillMaxWidth().focusRequester(typing).testTag("connected_apps.code"),
            enabled = !busy,
            textStyle = MaterialTheme.typography.bodyLarge.copy(fontFamily = FontFamily.Monospace, letterSpacing = 0.12.em),
            label = { Text("Or type the code") },
            placeholder = { Text("XXXXX-XXXXX") },
            isError = text.isNotBlank() && code == null && text.filter { it.isLetterOrDigit() }.length >= 10,
            inputTransformation = InputTransformation.maxLengthReject(16),
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters, autoCorrectEnabled = false, keyboardType = KeyboardType.Ascii, imeAction = ImeAction.Done),
            onKeyboardAction = { code?.let(::pair) },
            lineLimits = TextFieldLineLimits.SingleLine,
            shape = MaterialTheme.shapes.large,
        )
        Button(code?.takeIf { !busy }?.let { { pair(it) } }, Modifier.fillMaxWidth().testTag("connected_apps.pair_confirm"), variant = ButtonVariant.Outline) {
            if (busy) LoadingIndicator() else Text("Pair")
        }
        if (busy) Note(proofStage())
        Note("It gets its own key, which can't apply for outings unless you turn them on. You can revoke it here any time.")
    }
}

/** New MCP key: the outings switch, Create (screen lock), then the key once with the MCP URL. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun NewMcpSheet(onDone: () -> Unit, onDismiss: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val deviceAuth = rememberDeviceAuth()
    val haptics = rememberHaptics()
    var outings by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var made by remember { mutableStateOf<NewKey?>(null) }

    Sheet("connected_apps.mcp_sheet", onDismiss) {
        Text("New MCP key", style = MaterialTheme.typography.titleLarge)
        val key = made
        if (key == null) {
            Note("Add vitapmate to Claude or another AI app that speaks MCP. It can read your attendance, marks, timetable and other VTOP pages.")
            GroupItem(
                "Outings",
                "connected_apps.mcp_outings",
                null,
                "Let it apply for and cancel outings (it asks you first)",
                onClick = { outings = !outings; haptics.toggle(outings) },
                trailing = { ItemSwitch(outings, { outings = it; haptics.toggle(it) }) },
                on = outings,
            )
            Button(
                if (busy) null else {
                    {
                        deviceAuth.confirm("New MCP key") {
                            scope.launch {
                                busy = true
                                appsCall("new_mcp") { c, f -> c.createKey(f, outings, null) }
                                    .onSuccess { made = it; haptics.confirm(); onDone() }
                                    .onFailure { haptics.reject() }
                                busy = false
                            }
                        }
                    }
                },
                Modifier.fillMaxWidth().testTag("connected_apps.mcp_create"),
            ) { if (busy) LoadingIndicator() else Text("Create key") }
        } else {
            Copyable("Key", key.key, "connected_apps.mcp_key") { copy(context, "MCP key", key.key) }
            val server = rememberServer()
            val mcpUrl = key.mcpUrl.ifBlank { "$server/mcp" }
            Copyable("MCP URL", mcpUrl, "connected_apps.mcp_url") { copy(context, "MCP URL", mcpUrl) }
            val warning = AppTheme.tones.warning
            Row(Modifier.fillMaxWidth().background(warning.subtle, RoundedCornerShape(18.dp)).padding(horizontal = 14.dp, vertical = 12.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Icon(Lucide.TriangleAlert, null, Modifier.size(18.dp), tint = warning.base)
                Text("This key is shown only once. Copy it into your AI app now; if you lose it, revoke it and make another.", style = MaterialTheme.typography.bodyMedium, color = warning.onSubtle)
            }
            Button(onDismiss, Modifier.fillMaxWidth().testTag("connected_apps.mcp_done"), variant = ButtonVariant.Outline) { Text("Done") }
        }
    }
}

@Composable
private fun Copyable(label: String, value: String, tag: String, onCopy: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    Column(Modifier.fillMaxWidth().background(scheme.surfaceContainerHighest, MaterialTheme.shapes.large).padding(start = Space.lg, top = Space.md, bottom = Space.md, end = Space.xs)) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = scheme.onSurfaceVariant)
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(value, Modifier.weight(1f).testTag(tag), style = MaterialTheme.typography.bodyLarge.copy(fontFamily = FontFamily.Monospace))
            IconButton(onCopy, Modifier.testTag("$tag.copy")) { Icon(Lucide.Copy, "Copy $label") }
        }
    }
}

/** One key: kind, dates, the outings switch, its last uses, Revoke (asks first). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun KeySheet(key: AccessKey, onChanged: () -> Unit, onDismiss: () -> Unit) {
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    var outings by remember { mutableStateOf(key.outings) }
    var confirmRevoke by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }

    fun setOutings(on: Boolean) {
        if (busy) return
        scope.launch {
            busy = true
            appsCall("outings") { c, f -> c.setKeyOutings(f, key.id, on) }.onSuccess { outings = on; haptics.toggle(on) }
            busy = false
        }
    }

    Sheet("connected_apps.key_sheet", onDismiss, full = true) {
        Row(horizontalArrangement = Arrangement.spacedBy(14.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(52.dp).background(MaterialTheme.colorScheme.secondaryContainer, key.kind.shape), Alignment.Center) {
                Icon(key.kind.icon, null, Modifier.size(24.dp), tint = MaterialTheme.colorScheme.onSecondaryContainer)
            }
            Column {
                Text(key.label, style = MaterialTheme.typography.titleLarge)
                Note("${key.kind.label} · made ${ago(key.createdAt)}")
            }
        }
        Box(Modifier.clip(RoundedCornerShape(20.dp)).background(MaterialTheme.colorScheme.surfaceContainerHigh).padding(horizontal = 14.dp)) {
            GroupItem(
                "Outings",
                "connected_apps.key_outings",
                null,
                "Apply for and cancel outings with this key",
                enabled = !busy,
                onClick = { setOutings(!outings) },
                trailing = { ItemSwitch(outings, ::setOutings, enabled = !busy) },
                on = outings,
            )
        }
        Column {
            Text("Last uses", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
            if (key.uses.isEmpty()) Note("Not used yet.")
            else Column(Modifier.padding(top = 8.dp).clip(RoundedCornerShape(20.dp)).background(MaterialTheme.colorScheme.surfaceContainerHigh).padding(vertical = 4.dp)) {
                key.uses.forEachIndexed { i, use ->
                    TimelineItem(if (use.via == KeyKind.BROWSER) "Browser extension" else "AI app (MCP)", "connected_apps.use.$i", ago(use.at))
                }
            }
        }
        val danger = AppTheme.tones.danger
        androidx.compose.material3.Button(
            { confirmRevoke = true },
            Modifier.fillMaxWidth().height(54.dp).testTag("connected_apps.revoke"),
            enabled = !busy,
            shape = CircleShape,
            colors = ButtonDefaults.buttonColors(containerColor = danger.subtle, contentColor = danger.onSubtle),
        ) {
            Icon(Lucide.Trash2, null, Modifier.size(18.dp), tint = danger.base)
            Spacer(Modifier.width(10.dp))
            Text("Revoke", style = MaterialTheme.typography.titleSmallEmphasized)
        }
    }

    if (confirmRevoke) {
        AppDialog(
            { confirmRevoke = false },
            "Revoke ${key.label}?",
            "Revoke",
            {
                confirmRevoke = false
                scope.launch {
                    busy = true
                    appsCall("revoke") { c, f -> c.revokeKey(f, key.id) }.onSuccess {
                        haptics.confirm()
                        showToast("Key revoked", "${key.label} no longer works.")
                        onChanged()
                    }
                    busy = false
                }
            },
            icon = Lucide.Trash2,
            danger = true,
            confirmTag = "connected_apps.revoke.confirm",
        ) { Text("It stops working at once. To use it again, pair the browser or make a new key.") }
    }
}

/** The account's last VTOP sign-ins (phones and the server), live, with Clear history (asks first). */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun SignInsSheet(onDismiss: () -> Unit) {
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    var phase by remember { mutableStateOf<ServerLoad<List<ServerSignIn>>>(ServerLoad.Loading) }
    var confirmClear by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    fun load() {
        scope.launch { phase = phase.after(appsCall("sign_ins") { c, f -> c.serverSignIns(f) }) }
    }
    val change by rememberLive(LiveTopic.SIGN_INS, phase is ServerLoad.Ready)
    LaunchedEffect(change) { if (change > 0) load() }

    Sheet("connected_apps.sign_ins_sheet", onDismiss, full = true) {
        Text("Recent sign-ins", style = MaterialTheme.typography.titleLarge)
        Note("Each time a phone or the server signed in to VTOP for a connected app.")
        when (val p = phase) {
            ServerLoad.Loading -> LoadingIndicator(Modifier.testTag("connected_apps.sign_ins.loading"))
            is ServerLoad.Failed -> ProblemView(p.problem, onRetry = { phase = ServerLoad.Loading; load() }, Modifier.fillMaxWidth())
            is ServerLoad.Ready -> if (p.value.isEmpty()) Note("No sign-ins yet.") else {
                Column(Modifier.clip(RoundedCornerShape(20.dp)).background(MaterialTheme.colorScheme.surfaceContainerHigh).padding(vertical = 4.dp)) {
                    p.value.forEachIndexed { i, signIn ->
                        val failed = signIn.outcome as? SignInOutcome.Failed
                        TimelineItem(
                            "${if (failed == null) "Signed in" else "Failed"} on ${if (signIn.byServer) "the server" else "a phone"}",
                            "connected_apps.sign_in.$i",
                            ago(signIn.at),
                            failed?.reason,
                            failed = failed != null,
                        )
                    }
                }
                OutlinedButton({ confirmClear = true }, Modifier.fillMaxWidth().testTag("connected_apps.sign_ins.clear"), enabled = !busy) {
                    Icon(Lucide.History, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Clear history")
                }
            }
        }
    }

    if (confirmClear) {
        AppDialog(
            { confirmClear = false },
            "Clear the sign-in history?",
            "Clear",
            {
                confirmClear = false
                scope.launch {
                    busy = true
                    appsCall("clear_sign_ins") { c, f -> c.clearServerSignIns(f) }.onSuccess { done ->
                        if (done) {
                            haptics.confirm()
                            showToast("Sign-ins", "The history is cleared.")
                            load()
                        } else {
                            showToast("Sign-ins", "The server doesn't offer this yet.")
                        }
                    }.onFailure { haptics.reject() }
                    busy = false
                }
            },
            icon = Lucide.History,
            danger = true,
            confirmTag = "connected_apps.sign_ins.clear.confirm",
        ) { Text("The server forgets these sign-ins. New ones are recorded again.") }
    }
}
