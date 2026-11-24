package vitapmate.features.vtop

import android.content.Context
import android.net.Uri
import android.os.SystemClock
import android.text.format.Formatter
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import androidx.activity.OnBackPressedCallback
import androidx.activity.compose.LocalOnBackPressedDispatcherOwner
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.pulltorefresh.PullToRefreshDefaults
import androidx.compose.material3.pulltorefresh.rememberPullToRefreshState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import com.composables.icons.lucide.LogIn
import com.composables.icons.lucide.Lucide
import kotlin.coroutines.resume
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.drop
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.askedOtp
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppDialog
import vitapmate.ui.ProblemView
import vitapmate.ui.Space
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast

private const val SRC = VtopBrowser.SRC
private const val PREFS = "settings"
// The Flutter app's keys and defaults.
private const val COMPACT = "settings_vtop_compact_mode"
private const val DESKTOP = "settings_vtop_desktop_mode"

private class PrepareSlot(var job: Job? = null)

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
    var webState by remember { mutableStateOf(VtopWebState()) }
    val prepare = remember { PrepareSlot() }
    var dark by remember { mutableStateOf(background.luminance() < 0.5f) }
    var compact by remember { mutableStateOf(prefs.getBoolean(COMPACT, true)) }
    var desktop by remember { mutableStateOf(prefs.getBoolean(DESKTOP, false)) }
    var fileCallback by remember { mutableStateOf<ValueCallback<Array<Uri>>?>(null) }
    val files = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        fileCallback?.onReceiveValue(WebChromeClient.FileChooserParams.parseResult(result.resultCode, result.data))
        fileCallback = null
    }

    var user by remember { mutableStateOf<String?>(null) }
    var recent by remember { mutableStateOf(emptyList<VtopPage>()) }
    var searching by remember { mutableStateOf(false) }
    var sidebar by remember { mutableStateOf<List<VtopPage>?>(null) }
    var saved by remember { mutableStateOf<String?>(null) }
    val assets = remember { VtopAssets(context.cacheDir) }
    val pull = rememberPullToRefreshState()
    val pullPx = with(LocalDensity.current) { PullToRefreshDefaults.PositionalThreshold.toPx() }
    val haptics = rememberHaptics()
    LaunchedEffect(pull) { snapshotFlow { pull.distanceFraction >= 1f }.drop(1).collect { if (it) haptics.threshold() } }

    lateinit var browser: VtopBrowser

    fun dispatch(event: VtopWebEvent) {
        val transition = webState.reduce(event, SystemClock.elapsedRealtime())
        webState = transition.state
        for (effect in transition.effects) {
            when (effect) {
                VtopWebEffect.DestroyView -> browser.destroy()
                VtopWebEffect.CancelPrepare -> {
                    prepare.job?.cancel()
                    prepare.job = null
                }
                is VtopWebEffect.Prepare -> {
                    prepare.job?.cancel()
                    prepare.job = scope.launch {
                        var id = ""
                        try {
                            flow(effect.flowName) { flowId ->
                                id = flowId
                                browser.flow = flowId
                                // The web view shares the app's cookie store: a confirmed session is all it needs.
                                val fresh = withContext(Dispatchers.IO) { AppCore.await().webSession(flowId, effect.force) }
                                Log.info(SRC, "session ready", flowId, mapOf("force" to effect.force.toString()))
                                dispatch(VtopWebEvent.PrepareSucceeded(effect.requestId, fresh))
                            }
                        } catch (e: CancellationException) {
                            throw e
                        } catch (e: Throwable) {
                            askedOtp(e)
                            val problem = problemOf(e, id)
                            Log.warn(SRC, "not opened: ${e::class.simpleName}", id)
                            dispatch(VtopWebEvent.PrepareFailed(effect.requestId, problem))
                        }
                    }
                }
                is VtopWebEffect.OpenSession -> browser.open(effect.session)
            }
        }
    }

    fun reload(trigger: String) {
        scope.launch { flow("vtop.reload") { id -> Log.info(SRC, "reload", id, mapOf("trigger" to trigger)); browser.reload() } }
    }

    browser = remember {
        VtopBrowser(
            onLoginPage = { dispatch(VtopWebEvent.LoginPageHit) },
            onAuthenticated = { dispatch(VtopWebEvent.Authenticated) },
            onLost = { dispatch(VtopWebEvent.RenderProcessLost(browser.flow.orEmpty())) },
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
            if (wasReloading && browser.error == null && webState.phase is VtopWebPhase.Active) haptics.confirm()
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
    LaunchedEffect(Unit) { dispatch(VtopWebEvent.Open) }
    DisposableEffect(Unit) {
        onDispose {
            prepare.job?.cancel()
            browser.destroy()
        }
    }
    // The OTP was accepted elsewhere (the app's prompt): try again.
    val otpAtStart = remember { OtpPrompt.generation }
    LaunchedEffect(OtpPrompt.generation) {
        if (OtpPrompt.generation != otpAtStart) dispatch(VtopWebEvent.OtpResolved)
    }

    val phase = webState.phase
    val ready = phase is VtopWebPhase.Active
    // Back steps through VTOP's pages (closing an overlay first) while there is one to go back to;
    // otherwise it is the app's back (predictive) and the screen closes. The arrow always closes.
    val dispatcher = LocalOnBackPressedDispatcherOwner.current?.onBackPressedDispatcher
    val stepping = ready && browser.canStepBack
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

    Scaffold(
        topBar = {
            VtopToolbar(
                title = browser.page?.title ?: "VTOP",
                enabled = ready,
                onClose = callback::close,
                onHome = { browser.home() },
                saved = saved,
                onMenu = { scope.launch { saved = withContext(Dispatchers.IO) { Formatter.formatShortFileSize(context, assets.bytes()) } } },
                onClearSaved = {
                    scope.launch {
                        flow("vtop.clear_saved") { id ->
                            withContext(Dispatchers.IO) { assets.clear() }
                            Log.info(SRC, "saved files cleared", id)
                        }
                        showToast("Saved files cleared", "VTOP's files download again as the page reloads.")
                        reload("clear_saved")
                    }
                },
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
            val generation = webState.viewGeneration
            key(generation) {
                AndroidView(
                    factory = { ctx ->
                        browser.create(ctx, background.toArgb()).also { webView ->
                            webView.post {
                                if (browser.view === webView) dispatch(VtopWebEvent.ViewAttached(generation))
                            }
                        }
                    },
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
            when (phase) {
                is VtopWebPhase.Failed -> Box(Modifier.fillMaxSize().background(background), Alignment.Center) {
                    ProblemView(phase.problem, onRetry = { dispatch(VtopWebEvent.Retry) })
                }
                is VtopWebPhase.Crashed -> Box(Modifier.fillMaxSize().background(background), Alignment.Center) {
                    ProblemView(phase.problem, onRetry = { dispatch(VtopWebEvent.Retry) })
                }
                is VtopWebPhase.Preparing -> VtopLoading(phase.force)
                is VtopWebPhase.WaitingForView -> VtopLoading(phase.recovery == VtopRecovery.AutoRelogin)
                is VtopWebPhase.Active -> {}
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

    if (phase is VtopWebPhase.Active && phase.recovery == VtopRecovery.PromptRelogin) {
        AppDialog(
            { dispatch(VtopWebEvent.DismissReloginPrompt) },
            "Login expired",
            "Try again",
            { dispatch(VtopWebEvent.ConfirmRelogin) },
            Modifier.testTag("vtop.relogin.dialog"),
            icon = Lucide.LogIn,
            confirmTag = "vtop.relogin.confirm",
        ) { Text("VTOP could not restore your session. Try signing in again?") }
    }
}
