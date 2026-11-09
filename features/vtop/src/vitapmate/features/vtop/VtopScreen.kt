package vitapmate.features.vtop

import kotlinx.coroutines.flow.drop
import androidx.compose.runtime.snapshotFlow
import vitapmate.ui.rememberHaptics
import android.content.Context
import android.net.Uri
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import androidx.activity.compose.LocalOnBackPressedDispatcherOwner
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.OnBackPressedCallback
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Scaffold
import androidx.compose.material3.pulltorefresh.PullToRefreshDefaults
import androidx.compose.material3.pulltorefresh.rememberPullToRefreshState
import androidx.compose.ui.platform.LocalDensity
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.CoreException
import vitapmate.core.WebSession
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.Problem
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.ProblemView
import vitapmate.ui.Space

private const val SRC = VtopBrowser.SRC
private const val PREFS = "settings"
// The Flutter app's keys and defaults.
private const val COMPACT = "settings_vtop_compact_mode"
private const val DESKTOP = "settings_vtop_desktop_mode"

/**
 * VTOP itself, signed in with the app's session (the Flutter app's `VtopWebview`): Rust confirms
 * the session (signing in again when it expired) and hands its cookies to a fresh cookie jar; a
 * bounce to VTOP's login page signs in again once by itself, then asks. Back steps through VTOP
 * first.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun VtopScreen() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val prefs = remember { context.getSharedPreferences(PREFS, Context.MODE_PRIVATE) }
    val background = MaterialTheme.colorScheme.background
    var session by remember { mutableStateOf<WebSession?>(null) }
    var problem by remember { mutableStateOf<Problem?>(null) }
    var reconnecting by remember { mutableStateOf(false) }
    var autoTried by remember { mutableStateOf(false) }
    var askRelogin by remember { mutableStateOf(false) }
    var dark by remember { mutableStateOf(background.luminance() < 0.5f) }
    var compact by remember { mutableStateOf(prefs.getBoolean(COMPACT, true)) }
    var desktop by remember { mutableStateOf(prefs.getBoolean(DESKTOP, false)) }
    var viewGeneration by remember { mutableIntStateOf(0) }
    var lastLoss by remember { mutableStateOf(0L) }
    var fileCallback by remember { mutableStateOf<ValueCallback<Array<Uri>>?>(null) }
    val files = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        fileCallback?.onReceiveValue(WebChromeClient.FileChooserParams.parseResult(result.resultCode, result.data))
        fileCallback = null
    }

    var user by remember { mutableStateOf<String?>(null) }
    var recent by remember { mutableStateOf(emptyList<VtopPage>()) }
    var searching by remember { mutableStateOf(false) }
    var sidebar by remember { mutableStateOf<List<VtopPage>?>(null) }
    val pull = rememberPullToRefreshState()
    val pullPx = with(LocalDensity.current) { PullToRefreshDefaults.PositionalThreshold.toPx() }
    val haptics = rememberHaptics()
    LaunchedEffect(pull) { snapshotFlow { pull.distanceFraction >= 1f }.drop(1).collect { if (it) haptics.threshold() } }

    lateinit var browser: VtopBrowser

    /** Asks Rust for a confirmed session and opens VTOP's start page. */
    suspend fun prepare(name: String, force: Boolean) {
        var id = ""
        problem = null
        session = null
        try {
            flow(name) { flowId ->
                id = flowId
                browser.flow = flowId
                // The web view shares the app's cookie store: a confirmed session is all it needs.
                val fresh = withContext(Dispatchers.IO) { AppCore.await().webSession(flowId, force) }
                Log.info(SRC, "session ready", flowId, mapOf("force" to force.toString()))
                session = fresh
                browser.open(fresh)
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            if (e is CoreException.OtpRequired) OtpPrompt.ask(e.reason)
            problem = problemOf(e, id)
            Log.warn(SRC, "not opened: ${e::class.simpleName}", id)
        }
    }

    fun reload(trigger: String) {
        scope.launch { flow("vtop.reload") { id -> Log.info(SRC, "reload", id, mapOf("trigger" to trigger)); browser.reload() } }
    }

    fun relogin() {
        if (reconnecting) return
        reconnecting = true
        scope.launch {
            try {
                prepare("vtop.relogin", force = true)
            } finally {
                reconnecting = false
            }
        }
    }

    browser = remember {
        VtopBrowser(
            onLoginPage = {
                when {
                    reconnecting || askRelogin -> {}
                    !autoTried -> {
                        autoTried = true
                        relogin()
                    }
                    else -> askRelogin = true
                }
            },
            // A signed-in page starts a new recovery cycle; a sign-in alone doesn't (VTOP may bounce straight back).
            onAuthenticated = { autoTried = false },
            onLost = {
                // A page that keeps crashing stops here instead of looping.
                val now = System.currentTimeMillis()
                val repeated = now - lastLoss < 30_000
                lastLoss = now
                browser.destroy()
                viewGeneration++
                if (repeated) {
                    problem = problemOf(IllegalStateException("VTOP stopped responding"), browser.flow.orEmpty())
                } else {
                    scope.launch { prepare("vtop.open", force = false) }
                }
            },
            onChooseFile = { callback, params ->
                fileCallback?.onReceiveValue(null)
                fileCallback = callback
                runCatching { files.launch(params.createIntent()) }
                    .onFailure { fileCallback = null; callback.onReceiveValue(null) }
                    .isSuccess
            },
        )
    }
    browser.onVisit = { page -> recent = VtopRecent.add(context, user, page) }
    browser.onPull = { d -> scope.launch { pull.snapTo((d / pullPx).coerceAtMost(1.5f)) } }
    browser.onPullEnd = { d ->
        if (d >= pullPx) reload("pull") else scope.launch { pull.animateToHidden() }
    }
    // A reload that finished without an error: a light confirm, then the pull indicator goes.
    var wasReloading by remember { mutableStateOf(false) }
    LaunchedEffect(browser.reloading) {
        if (browser.reloading) {
            wasReloading = true
        } else {
            if (wasReloading && browser.error == null) haptics.confirm()
            wasReloading = false
            pull.animateToHidden()
        }
    }
    browser.dark = dark
    browser.compact = compact
    browser.desktop = desktop

    LaunchedEffect(Unit) {
        user = withContext(Dispatchers.IO) { runCatching { AppCore.await().signedInUser("") }.getOrNull() }
        recent = VtopRecent.read(context, user)
    }
    LaunchedEffect(Unit) { prepare("vtop.open", force = false) }
    DisposableEffect(Unit) { onDispose { browser.destroy() } }
    // The OTP was accepted elsewhere (the app's prompt): try again.
    val otpAtStart = remember { OtpPrompt.generation }
    LaunchedEffect(OtpPrompt.generation) {
        if (OtpPrompt.generation != otpAtStart && session == null) prepare("vtop.open", force = false)
    }

    // Back steps through VTOP's pages (closing an overlay first) while there is one to go back to;
    // otherwise it is the app's back (predictive) and the screen closes. The arrow always closes.
    val dispatcher = LocalOnBackPressedDispatcherOwner.current?.onBackPressedDispatcher
    val stepping = session != null && browser.canStepBack
    val callback = remember {
        object : OnBackPressedCallback(false) {
            override fun handleOnBackPressed() = browser.back { handled -> if (!handled) close() }

            fun close() {
                isEnabled = false
                dispatcher?.onBackPressed()
            }
        }
    }
    callback.isEnabled = stepping
    DisposableEffect(dispatcher) {
        dispatcher?.addCallback(callback)
        onDispose { callback.remove() }
    }

    val ready = session != null && problem == null
    Scaffold(
        topBar = {
            VtopToolbar(
            title = browser.page?.title ?: "VTOP",
            enabled = ready,
            onClose = callback::close,
            onHome = { browser.home() },
            onReload = { reload("menu") },
            onSearch = {
                sidebar = null
                searching = true
                scope.launch {
                    flow("vtop.search") { id ->
                        val pages = suspendCancellableCoroutine { done -> browser.pages { done.resume(it) } }
                        sidebar = pages
                        Log.info(SRC, "search pages", id, mapOf("pages" to pages.size.toString(), "recent" to recent.size.toString()))
                    }
                }
            },
            toggles = listOf(
                VtopToggle("dark", "Dark pages", dark) { dark = !dark; browser.dark = dark; browser.applyPreferences() },
                VtopToggle("compact", "Compact view", compact) {
                    compact = !compact
                    prefs.edit().putBoolean(COMPACT, compact).apply()
                    browser.compact = compact
                    browser.applyPreferences()
                },
                VtopToggle("desktop", "Desktop mode", desktop) {
                    desktop = !desktop
                    prefs.edit().putBoolean(DESKTOP, desktop).apply()
                    browser.desktop = desktop
                    browser.applyPreferences()
                },
            ),
            )
        },
        containerColor = background,
    ) { padding ->
        Box(Modifier.fillMaxSize().padding(padding)) {
            // The view starts while the session is prepared, so its start-up overlaps the network work.
            key(viewGeneration) {
                AndroidView(
                    factory = { browser.create(it, background.toArgb()) },
                    modifier = Modifier.fillMaxSize().testTag("vtop.web"),
                )
            }
            if (ready) PullToRefreshDefaults.LoadingIndicator(pull, browser.reloading, Modifier.align(Alignment.TopCenter).testTag("vtop.pull"))
            if (ready && browser.busy) {
                val bar = Modifier.fillMaxWidth().align(Alignment.TopCenter).testTag("vtop.progress")
                // Page loads show how far they are; VTOP's in-place requests only that it is busy.
                if (browser.pageLoading && browser.progress in 1..99) LinearWavyProgressIndicator({ browser.progress / 100f }, bar)
                else LinearWavyProgressIndicator(bar)
            }
            if (ready && !browser.hasContent) {
                Box(Modifier.fillMaxSize().background(background), Alignment.Center) { LoadingIndicator() }
            }
            val notice = browser.error
            if (ready && notice != null) {
                Column(
                    Modifier.fillMaxWidth().padding(Space.md).clip(MaterialTheme.shapes.large)
                        .background(MaterialTheme.colorScheme.surfaceContainerHigh)
                        .border(1.dp, MaterialTheme.colorScheme.outlineVariant, MaterialTheme.shapes.large)
                        .padding(Space.lg).testTag("vtop.notice"),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Text(notice, style = MaterialTheme.typography.bodyMedium, textAlign = TextAlign.Center)
                    OutlinedButton({ browser.home(reload = true) }, Modifier.padding(top = Space.md).testTag("vtop.notice.home")) { Text("Open Home") }
                }
            }
            when {
                problem != null -> Box(Modifier.fillMaxSize().background(background), Alignment.Center) {
                    ProblemView(problem!!, onRetry = { scope.launch { prepare("vtop.open", force = false) } })
                }
                session == null -> VtopLoading(reconnecting)
            }
        }
    }

    if (searching) {
        VtopSearch(sidebar, recent, onSelect = { page ->
            searching = false
            scope.launch {
                flow("vtop.open_page") { id ->
                    Log.info(SRC, "open page", id, mapOf("menu" to VtopBrowser.pathOf(page.url), "recent" to (page in recent).toString()))
                    browser.openMenu(page.url)
                }
            }
        }, onDismiss = { searching = false })
    }

    if (askRelogin) {
        AlertDialog(
            onDismissRequest = { askRelogin = false },
            title = { Text("Login expired") },
            text = { Text("VTOP could not restore your session. Try signing in again?") },
            confirmButton = {
                TextButton({ askRelogin = false; relogin() }, Modifier.testTag("vtop.relogin.confirm")) { Text("Try again") }
            },
            dismissButton = { TextButton({ askRelogin = false }) { Text("Cancel") } },
            modifier = Modifier.testTag("vtop.relogin.dialog"),
        )
    }
}
