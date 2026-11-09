package vitapmate

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import com.composables.icons.lucide.Globe
import com.composables.icons.lucide.KeyRound
import com.composables.icons.lucide.Download
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Scan
import com.composables.icons.lucide.Sparkles
import com.composables.icons.lucide.Trash2
import com.composables.icons.lucide.TriangleAlert
import com.google.mlkit.vision.barcode.common.Barcode
import com.google.mlkit.vision.codescanner.GmsBarcodeScannerOptions
import com.google.mlkit.vision.codescanner.GmsBarcodeScanning
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.AccessKey
import vitapmate.core.ConnectedApps
import vitapmate.core.CoreException
import vitapmate.core.Core
import vitapmate.core.KeyKind
import vitapmate.core.NewKey
import vitapmate.core.pairCode
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.problemOf
import vitapmate.features.login.ago
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonVariant
import vitapmate.ui.OpenHaptic
import vitapmate.ui.ProblemView
import vitapmate.ui.Space
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast
import vitapmate.data.Problem

/** Route id of Settings → Account → Connected apps. */
const val CONNECTED_APPS = "connected_apps"

/** The extension's latest build (Chrome zip and signed Firefox add-on), replaced by CI. */
private const val EXTENSION_URL = "https://github.com/itsAster0/vitap-mate/releases/tag/extension-latest"

private const val MCP_URL = "https://vmate.aster0.dev/mcp"

/**
 * Connected apps (Rust's `keys.rs`): the access keys of the browser extension and AI apps (MCP),
 * each with its last uses, outings switch and revoke; "Pair a browser" (scan the extension's QR
 * code or type its code) and "New MCP key" (shown once). Making a key asks for the screen lock
 * and links this phone to the server account the first time. Flows `connected_apps.*`, lines
 * `ui.connected_apps`; keys, codes and cookies are never logged.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ConnectedAppsScreen() {
    val scope = rememberCoroutineScope()
    val uri = LocalUriHandler.current
    var apps by remember { mutableStateOf<ConnectedApps?>(null) }
    var problem by remember { mutableStateOf<Problem?>(null) }
    var loading by remember { mutableStateOf(false) }
    var pairing by remember { mutableStateOf(false) }
    var newMcp by remember { mutableStateOf(false) }
    var opened by remember { mutableStateOf<AccessKey?>(null) }

    fun load() {
        scope.launch {
            loading = true
            run("load") { c, f -> c.connectedApps(f) }
                .onSuccess { apps = it; problem = null }
                .onFailure { e -> if (apps == null) problem = problemOf(e, "") }
            loading = false
        }
    }
    LaunchedEffect(Unit) { load() }

    Page("Connected apps", CONNECTED_APPS) { padding ->
        LazyColumn(contentPadding = padding) {
            item {
                ListGroup("Add", footer = "The browser extension signs you in to VTOP in Chrome or Firefox; AI apps such as Claude read your VTOP data over MCP. Neither sees your password.") {
                    GroupItem("Pair a browser", "connected_apps.pair", Lucide.Globe, "Scan the code the vitapmate extension shows", onClick = { pairing = true }, trailing = { Chevron() })
                    GroupItem("New MCP key", "connected_apps.new_mcp", Lucide.Sparkles, "For Claude and other AI apps", onClick = { newMcp = true }, trailing = { Chevron() })
                    GroupItem("Get the extension", "connected_apps.get_extension", Lucide.Download, "Chrome zip and Firefox add-on, from GitHub", onClick = { uri.openUri(EXTENSION_URL) }, trailing = { Icon(Lucide.ExternalLink, null, tint = MaterialTheme.colorScheme.onSurfaceVariant) })
                }
            }
            val current = apps
            when {
                current != null -> item {
                    ListGroup("Keys", footer = "Up to 10 keys. Revoke one you no longer use; it stops working at once.") {
                        if (current.keys.isEmpty()) GroupItem("No keys yet", "connected_apps.empty", Lucide.KeyRound, "Pair a browser or make an MCP key above")
                        current.keys.forEach { key ->
                            GroupItem(key.label, "connected_apps.key.${key.id}", key.kind.icon, keySummary(key), onClick = { opened = key }, trailing = { Chevron() })
                        }
                    }
                }
                problem != null -> item { ProblemView(problem!!, onRetry = ::load, Modifier.fillMaxWidth(), busy = loading) }
                else -> item { Box(Modifier.fillMaxWidth().padding(Space.xl), Alignment.Center) { LoadingIndicator(Modifier.testTag("connected_apps.loading")) } }
            }
        }
    }

    if (pairing) PairSheet(onDone = { pairing = false; load() }, onDismiss = { pairing = false })
    if (newMcp) NewMcpSheet(onDone = { load() }, onDismiss = { newMcp = false })
    opened?.let { key -> KeySheet(key, onChanged = { opened = null; load() }, onDismiss = { opened = null }) }
}

private val KeyKind.icon get() = if (this == KeyKind.BROWSER) Lucide.Globe else Lucide.Sparkles
private val KeyKind.label get() = if (this == KeyKind.BROWSER) "Browser" else "MCP"

private fun keySummary(key: AccessKey) = buildList {
    add(key.kind.label)
    add(key.lastUsedAt?.let { "used ${ago(it)}" } ?: "never used")
    if (key.outings) add("outings on")
}.joinToString(" · ")

/** One call as flow `connected_apps.<name>`, off the main thread; a failure is logged and toasted. */
private suspend fun <T> run(name: String, call: suspend (Core, String) -> T): Result<T> {
    var flowId = ""
    return try {
        Result.success(flow("connected_apps.$name") { f -> flowId = f; withContext(Dispatchers.IO) { call(AppCore.await(), f) } })
    } catch (e: CancellationException) {
        throw e
    } catch (e: Throwable) {
        Log.warn("ui.connected_apps", "$name failed: ${e::class.simpleName}", flowId)
        when (e) {
            is CoreException.OtpRequired -> OtpPrompt.ask(e.reason)
            is CoreException.Other -> showToast("Connected apps", e.reason)
            else -> showToast(problemOf(e, flowId))
        }
        Result.failure(e)
    }
}

/** `ABCDE12345` as the extension shows it. */
private fun shown(code: String) = "${code.take(5)}-${code.drop(5)}"

private fun copy(context: Context, label: String, text: String) {
    context.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText(label, text))
    showToast("Copied", label)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun Sheet(tag: String, onDismiss: () -> Unit, content: @Composable () -> Unit) {
    val sheet = rememberBottomSheetState(initialValue = SheetValue.Hidden, enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded))
    OpenHaptic()
    ModalBottomSheet(onDismiss, Modifier.testTag(tag), sheet, containerColor = MaterialTheme.colorScheme.surfaceContainerLow) {
        Column(
            Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(horizontal = Space.xl).padding(bottom = Space.xl),
            verticalArrangement = Arrangement.spacedBy(Space.lg),
        ) { content() }
    }
}

@Composable
private fun Note(text: String) = Text(text, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)

/**
 * Pair a browser: Scan QR (Play services' scanner: the code, or a link ending in `#code`) or type
 * the code; then the screen lock, then the key goes to the extension through the edge.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun PairSheet(onDone: () -> Unit, onDismiss: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val deviceAuth = rememberDeviceAuth()
    val haptics = rememberHaptics()
    val typing = remember { FocusRequester() }
    var text by remember { mutableStateOf("") }
    var busy by remember { mutableStateOf(false) }
    val code = pairCode(text)

    fun pair(code: String) {
        deviceAuth.confirm("Pair a browser") {
            scope.launch {
                busy = true
                run("pair") { c, f -> c.createKey(f, false, code) }
                    .onSuccess {
                        haptics.confirm()
                        showToast("Browser paired", "The extension signs in to VTOP by itself now.")
                        onDone()
                    }
                    .onFailure { haptics.reject() }
                busy = false
            }
        }
    }

    fun scan() {
        val options = GmsBarcodeScannerOptions.Builder().setBarcodeFormats(Barcode.FORMAT_QR_CODE).build()
        val unavailable = { why: String ->
            Log.info("ui.connected_apps", "scanner unavailable", fields = mapOf("why" to why))
            showToast("Scanner not ready", "Type the code the extension shows instead.")
            runCatching { typing.requestFocus() }
        }
        runCatching {
            GmsBarcodeScanning.getClient(context, options).startScan()
                .addOnSuccessListener { barcode ->
                    val scanned = pairCode(barcode.rawValue.orEmpty())
                    Log.info("ui.connected_apps", "scanned", fields = mapOf("pair_code" to "${scanned != null}"))
                    if (scanned == null) {
                        showToast("Not a pairing code", "Scan the QR code the vitapmate extension shows.")
                    } else {
                        text = shown(scanned)
                        pair(scanned)
                    }
                }
                .addOnFailureListener { unavailable(it::class.simpleName ?: "error") }
        }.onFailure { unavailable(it::class.simpleName ?: "error") }
    }

    Sheet("connected_apps.pair_sheet", onDismiss) {
        Text("Pair a browser", style = MaterialTheme.typography.titleLarge)
        Note("Open the vitapmate extension in Chrome or Firefox. It shows a QR code and a 10-character code.")
        Button(if (busy) null else ::scan, Modifier.fillMaxWidth().testTag("connected_apps.scan"), prefix = Lucide.Scan) { Text("Scan QR code") }
        OutlinedTextField(
            text,
            { text = it.take(16) },
            Modifier.fillMaxWidth().focusRequester(typing).testTag("connected_apps.code"),
            enabled = !busy,
            label = { Text("Or type the code") },
            placeholder = { Text("XXXXX-XXXXX") },
            isError = text.isNotBlank() && code == null && text.filter { it.isLetterOrDigit() }.length >= 10,
            keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Characters, autoCorrectEnabled = false, keyboardType = KeyboardType.Ascii, imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { code?.let(::pair) }),
            singleLine = true,
            textStyle = MaterialTheme.typography.bodyLarge.copy(fontFamily = FontFamily.Monospace),
            shape = MaterialTheme.shapes.large,
        )
        Button(code?.takeIf { !busy }?.let { { pair(it) } }, Modifier.fillMaxWidth().testTag("connected_apps.pair_confirm"), variant = ButtonVariant.Outline) {
            if (busy) LoadingIndicator() else Text("Pair")
        }
        Note("The extension gets its own key; it can't apply for outings. You can revoke it here any time.")
    }
}

/** New MCP key: the outings switch, Create (screen lock), then the key once with the MCP URL. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun NewMcpSheet(onDone: () -> Unit, onDismiss: () -> Unit) {
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
                trailing = { Switch(outings, { outings = it; haptics.toggle(it) }) },
            )
            Button(
                if (busy) null else {
                    {
                        deviceAuth.confirm("New MCP key") {
                            scope.launch {
                                busy = true
                                run("new_mcp") { c, f -> c.createKey(f, outings, null) }
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
            Copyable("MCP URL", key.mcpUrl.ifBlank { MCP_URL }, "connected_apps.mcp_url") { copy(context, "MCP URL", key.mcpUrl.ifBlank { MCP_URL }) }
            Row(horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
                Icon(Lucide.TriangleAlert, null, tint = AppTheme.tones.warning.base)
                Note("This key is shown only once. Copy it into your AI app now; if you lose it, revoke it and make another.")
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
private fun KeySheet(key: AccessKey, onChanged: () -> Unit, onDismiss: () -> Unit) {
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    var outings by remember { mutableStateOf(key.outings) }
    var confirmRevoke by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }

    fun setOutings(on: Boolean) {
        if (busy) return
        scope.launch {
            busy = true
            run("outings") { c, f -> c.setKeyOutings(f, key.id, on) }.onSuccess { outings = on; haptics.toggle(on) }
            busy = false
        }
    }

    Sheet("connected_apps.key_sheet", onDismiss) {
        Text(key.label, style = MaterialTheme.typography.titleLarge)
        Note("${key.kind.label} · made ${ago(key.createdAt)}")
        GroupItem(
            "Outings",
            "connected_apps.key_outings",
            null,
            "Apply for and cancel outings with this key",
            enabled = !busy,
            onClick = { setOutings(!outings) },
            trailing = { Switch(outings, ::setOutings, enabled = !busy) },
        )
        Column {
            Text("Last uses", style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary)
            if (key.uses.isEmpty()) Note("Not used yet.")
            key.uses.forEachIndexed { i, use ->
                GroupItem(if (use.via == KeyKind.BROWSER) "Browser extension" else "AI app (MCP)", "connected_apps.use.$i", use.via.icon, trailing = { TrailingValue(ago(use.at)) })
            }
        }
        Button(if (busy) null else { { confirmRevoke = true } }, Modifier.fillMaxWidth().testTag("connected_apps.revoke"), variant = ButtonVariant.Outline, prefix = Lucide.Trash2) {
            Text("Revoke", color = MaterialTheme.colorScheme.error)
        }
    }

    if (confirmRevoke) {
        AlertDialog(
            onDismissRequest = { confirmRevoke = false },
            title = { Text("Revoke ${key.label}?") },
            text = { Text("It stops working at once. To use it again, pair the browser or make a new key.") },
            confirmButton = {
                TextButton(
                    {
                        confirmRevoke = false
                        scope.launch {
                            busy = true
                            run("revoke") { c, f -> c.revokeKey(f, key.id) }.onSuccess {
                                haptics.confirm()
                                showToast("Key revoked", "${key.label} no longer works.")
                                onChanged()
                            }
                            busy = false
                        }
                    },
                    Modifier.testTag("connected_apps.revoke.confirm"),
                ) { Text("Revoke", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = { TextButton({ confirmRevoke = false }) { Text("Cancel") } },
        )
    }
}
