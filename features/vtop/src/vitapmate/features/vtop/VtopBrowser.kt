package vitapmate.features.vtop

import android.annotation.SuppressLint
import android.app.DownloadManager
import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.content.pm.ApplicationInfo
import android.graphics.Bitmap
import android.net.Uri
import android.os.Environment
import android.os.SystemClock
import android.view.MotionEvent
import android.view.ViewGroup
import android.webkit.CookieManager
import android.webkit.JavascriptInterface
import android.webkit.RenderProcessGoneDetail
import android.webkit.URLUtil
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.resume
import kotlinx.coroutines.suspendCancellableCoroutine
import org.json.JSONArray
import vitapmate.core.WebSession
import vitapmate.data.Docs
import vitapmate.observability.Log
import vitapmate.ui.ToastTone
import vitapmate.ui.showToast

/** A page in VTOP's sidebar, under its sidebar [section]; [url] "" is VTOP's Home. */
internal data class VtopPage(val url: String, val title: String, val section: String = "")

/**
 * The web view and what the screen draws from it: progress, the page shown, the in-VTOP back
 * history (VTOP loads its pages in place, so the web view's own history stays short). Port of
 * the Flutter app's `vtop_webview_body.dart`.
 */
@Stable
internal class VtopBrowser(
    /** VTOP sent the view to its login page: the session is gone. */
    private val onLoginPage: () -> Unit,
    /** A signed-in page (with its sidebar) is showing. */
    private val onAuthenticated: () -> Unit,
    /** The page's process died; this view can't be used any more. */
    private val onLost: () -> Unit,
    /** A page asked for a file (`<input type=file>`). */
    private val onChooseFile: (ValueCallback<Array<Uri>>, WebChromeClient.FileChooserParams) -> Boolean,
) {
    var view: WebView? = null
        private set
    var progress by mutableIntStateOf(0)
        private set
    var pageLoading by mutableStateOf(true)
        private set
    /** Requests VTOP's page has open (its in-place navigation). */
    var requests by mutableIntStateOf(0)
        private set
    var hasContent by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
        private set
    /** The page shown, null for Home. */
    var page by mutableStateOf<VtopPage?>(null)
        private set
    /** Back has somewhere to go inside VTOP (a previous page); drives the back callback. */
    var canStepBack by mutableStateOf(false)
        private set
    var dark = false
    var compact = true
    var desktop = false
    /** The flow that opened the view, on its log lines. */
    var flow: String? = null
    /** A reload runs (pull-to-refresh shows until the page finishes). */
    var reloading by mutableStateOf(false)
        private set
    /** Pull-to-refresh: how far the finger pulled down from the top (px); then where it let go. */
    var onPull: (Float) -> Unit = {}
    var onPullEnd: (Float) -> Unit = {}
    /** Nothing inside the page is scrolled down ([VtopScripts.SCROLL]). */
    private var innerAtTop = true
    /** A sidebar page was opened (for the recent pages). */
    var onVisit: (VtopPage) -> Unit = {}
    /** The sidebar's pages, read once per loaded document. */
    private var sidebar: List<VtopPage>? = null

    private val history = mutableListOf<VtopPage>()
    private var restoring: String? = null
    private var menuRequest: String? = null
    private var menuSequence = 0
    private var startUrl: String? = null
    private var pageStart = 0L

    val busy get() = pageLoading || requests > 0

    /**
     * The one web view, made on first use. A host recreated around it (nav transitions move the
     * screen) gets the same view and page back.
     */
    fun create(context: Context, background: Int): WebView =
        view?.also { (it.parent as? ViewGroup)?.removeView(it) } ?: make(context, background)

    @SuppressLint("SetJavaScriptEnabled", "ClickableViewAccessibility")
    private fun make(context: Context, background: Int): WebView = WebView(context).apply {
        // As the Flutter app (isInspectable: kDebugMode): chrome://inspect in debug builds only.
        WebView.setWebContentsDebuggingEnabled(context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0)
        // AndroidView's default WRAP_CONTENT puts the WebView in its wrap-content height mode, where
        // CSS `100vh` is 0: VTOP's whole page (`#page_outline.vh-100`) collapses to nothing.
        layoutParams = ViewGroup.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.MATCH_PARENT)
        setBackgroundColor(background)
        settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            cacheMode = WebSettings.LOAD_DEFAULT
            // VTOP's viewport tag decides the width (desktop mode rewrites it); pinch zoom for its tables.
            useWideViewPort = true
            loadWithOverviewMode = true
            builtInZoomControls = true
            displayZoomControls = false
            setSupportMultipleWindows(false)
        }
        addJavascriptInterface(Bridge(), "vitapmate")
        webViewClient = Client()
        webChromeClient = object : WebChromeClient() {
            override fun onProgressChanged(view: WebView, newProgress: Int) {
                this@VtopBrowser.progress = newProgress
            }

            override fun onShowFileChooser(view: WebView, callback: ValueCallback<Array<Uri>>, params: FileChooserParams) =
                onChooseFile(callback, params)
        }
        var startY = 0f
        var pulling = false
        // Taps and scrolls still reach the page; only a downward drag that starts at the very top pulls.
        setOnTouchListener { _, event ->
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    startY = event.y
                    pulling = !canScrollVertically(-1) && innerAtTop && !reloading
                }
                MotionEvent.ACTION_MOVE -> if (pulling) onPull((event.y - startY).coerceAtLeast(0f))
                MotionEvent.ACTION_UP -> if (pulling) onPullEnd(event.y - startY).also { pulling = false }
                MotionEvent.ACTION_CANCEL -> if (pulling) onPullEnd(0f).also { pulling = false }
            }
            false
        }
        setDownloadListener { url, userAgent, disposition, mime, _ -> download(context, url, userAgent, disposition, mime) }
        view = this
    }

    /** When the screen leaves, or the page's process died (a new view is made next). */
    fun destroy() {
        view?.destroy()
        view = null
    }

    /** Loads [session]'s start page from scratch (after fresh cookies were set). */
    fun open(session: WebSession) {
        startUrl = session.startUrl
        home(reload = true)
    }

    /** VTOP's Home: in place when the page has it, else loaded again. */
    fun home(reload: Boolean = false) {
        val view = view ?: return
        val start = startUrl ?: return
        error = null
        if (!reload) {
            view.evaluateJavascript(VtopScripts.HOME) { opened ->
                if (opened == "true") visited("", "") else home(reload = true)
            }
            return
        }
        history.clear()
        history += VtopPage("", "")
        page = null
        sidebar = null
        stepBack()
        view.loadUrl(start)
    }

    fun reload() {
        error = null
        reloading = view != null
        view?.reload()
    }

    /** Applies the view preferences to the page shown now (new pages get them on load). */
    fun applyPreferences() {
        view?.evaluateJavascript(VtopScripts.preferences(dark, compact, desktop), null)
    }

    /** Flutter's document-start scripts: the bridge shim, activity, preferences (each runs once per document). */
    private fun inject(view: WebView) {
        view.evaluateJavascript(VtopScripts.BRIDGE, null)
        view.evaluateJavascript(VtopScripts.ACTIVITY, null)
        view.evaluateJavascript(VtopScripts.SCROLL, null)
        applyPreferences()
    }

    private fun stepBack() {
        canStepBack = history.size >= 2 || previousDocument() != null
    }

    /**
     * Steps (negative) back to the web view's previous document, when it is another VTOP page. Earlier loads of the same
     * page (the start page loaded again, a reload) and the login page don't count: Home is the root.
     */
    private fun previousDocument(): Int? {
        val list = view?.copyBackForwardList() ?: return null
        val current = list.currentItem?.url?.let { Uri.parse(it).path }
        val index = (list.currentIndex - 1 downTo 0).firstOrNull { Uri.parse(list.getItemAtIndex(it).url).path != current } ?: return null
        val uri = Uri.parse(list.getItemAtIndex(index).url)
        return (index - list.currentIndex).takeIf { isVtop(uri) && !uri.path.orEmpty().startsWith("/vtop/login") }
    }

    /** Every page in VTOP's sidebar ([VtopScripts.PAGES]); empty until a signed-in page is open. */
    fun pages(done: (List<VtopPage>) -> Unit) {
        sidebar?.takeIf { it.isNotEmpty() }?.let { return done(it) }
        val view = view ?: return done(emptyList())
        view.evaluateJavascript(VtopScripts.PAGES) { json ->
            val list = runCatching { JSONArray(json) }.getOrNull()
            val pages = (0 until (list?.length() ?: 0)).mapNotNull { i ->
                val o = list!!.optJSONObject(i) ?: return@mapNotNull null
                VtopPage(o.optString("url"), o.optString("title"), o.optString("section")).takeIf { it.url.isNotEmpty() && it.title.isNotEmpty() }
            }
            if (pages.isNotEmpty()) sidebar = pages
            done(pages)
        }
    }

    /** Opens the sidebar page [url] (Flutter's `openMenu`); a notice when VTOP no longer has it. */
    fun openMenu(url: String) {
        val view = view ?: return
        val request = "${++menuSequence}"
        menuRequest = request
        view.evaluateJavascript(VtopScripts.waitForMenu(url, request), null)
        view.postDelayed({ if (menuRequest == request) menuFailed() }, 11_000)
    }

    private fun menuFailed() {
        menuRequest = null
        error = "This VTOP menu is unavailable. Try opening Home."
    }

    /**
     * Back inside VTOP: closes an overlay, else reopens the previous sidebar page, else the web
     * view's own history. [done] gets false when there was nothing to go back to.
     */
    fun back(done: (Boolean) -> Unit) {
        val view = view ?: return done(false)
        view.evaluateJavascript(VtopScripts.CLOSE_OVERLAY) { closed ->
            if (closed == "true") return@evaluateJavascript done(true)
            if (history.size >= 2) {
                history.removeAt(history.lastIndex)
                val target = history.last()
                page = target.takeIf { it.url.isNotEmpty() }
                if (target.url.isEmpty()) {
                    home()
                } else {
                    restoring = target.url
                    openMenu(target.url)
                }
                stepBack()
                return@evaluateJavascript done(true)
            }
            val steps = previousDocument()
            if (steps != null) {
                view.goBackOrForward(steps)
                done(true)
            } else {
                done(false)
            }
        }
    }

    private fun visited(url: String, title: String) {
        if (url.contains("/download", ignoreCase = true)) return
        val restored = restoring
        restoring = null
        if (restored != null && restored == url) return
        val visit = VtopPage(url, title.ifEmpty { "VTOP" }, sidebar?.firstOrNull { it.url == url }?.section.orEmpty())
        when {
            // Home is the root; nothing before it is worth returning to.
            url.isEmpty() -> history.clear().also { history += visit }
            history.lastOrNull()?.url == url -> history[history.lastIndex] = visit
            else -> {
                history += visit
                if (history.size > 30) history.removeAt(0)
            }
        }
        page = visit.takeIf { url.isNotEmpty() }
        stepBack()
        innerAtTop = true
        if (url.isNotEmpty()) onVisit(visit)
        Log.info(SRC, "page", flow, mapOf("menu" to pathOf(url), "depth" to history.size.toString()))
    }

    private fun download(context: Context, url: String, userAgent: String?, disposition: String?, mime: String?) {
        val name = URLUtil.guessFileName(url, disposition, mime)
        runCatching {
            val request = DownloadManager.Request(Uri.parse(url))
                .addRequestHeader("Cookie", CookieManager.getInstance().getCookie(url).orEmpty())
                .addRequestHeader("User-Agent", userAgent.orEmpty())
                .addRequestHeader("Referer", view?.url.orEmpty())
                .setMimeType(mime)
                .setTitle(name)
                // Progress only: the finished file moves on, so a "completed" tap would find nothing.
                .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE)
                // The app's own folder first; Docs.watchDownload moves it to the download folder.
                .setDestinationInExternalFilesDir(context, Environment.DIRECTORY_DOWNLOADS, name)
            val id = context.getSystemService(DownloadManager::class.java).enqueue(request)
            // Into Docs and the download folder; outing pages' files as "outing" (the Flutter app's `_isOutingDownload`).
            val outing = listOfNotNull(page?.url, url, view?.url).any { it.lowercase().let { u -> "studentweekendouting" in u || "studentgeneralouting" in u } }
            Docs.watchDownload(context, id, name, if (outing) "outing" else "vtop")
        }.fold(
            {
                Log.info(SRC, "download started", flow, mapOf("path" to pathOf(url), "mime" to mime.orEmpty()))
                showToast("Downloading", "$name is saving to your download folder.")
            },
            { e ->
                Log.warn(SRC, "download failed: ${e::class.simpleName}", flow, mapOf("path" to pathOf(url)))
                showToast("Download failed", "Please try the download again.", tone = ToastTone.Error)
            },
        )
    }

    /** flutter_inappwebview's `callHandler` from the scripts ([VtopScripts.BRIDGE]), on the bridge thread. */
    private inner class Bridge {
        @JavascriptInterface
        fun call(name: String, json: String) {
            val args = runCatching { JSONArray(json) }.getOrNull() ?: return
            view?.post { handle(name, args) }
        }
    }

    private fun handle(name: String, args: JSONArray) {
        when (name) {
            "vtopScroll" -> innerAtTop = args.optBoolean(0, true)
            "vtopMenuChanged" -> visited(args.optString(0), args.optString(1))
            "vtopMenuResult" -> if (args.optString(0) == menuRequest) {
                if (args.optBoolean(1)) menuRequest = null else menuFailed()
            }
            "vtopActivity" -> {
                val event = args.optJSONObject(0) ?: return
                when (event.optString("type")) {
                    "activity" -> requests = event.optInt("active")
                    "ready" -> view?.evaluateJavascript(VtopScripts.AUTHENTICATED) { if (it == "true") onAuthenticated() }
                    // Aggregate numbers only, never URLs, resource names or response bodies.
                    "timing" -> Log.info(SRC, "resource timing", flow, listOf("duration", "resources", "transferBytes", "zeroTransfer").associateWith { event.optLong(it).toString() })
                }
            }
        }
    }

    private inner class Client : WebViewClient() {
        override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
            val uri = request.url
            if (isVtop(uri)) return false
            Log.info(SRC, "left VTOP", flow, mapOf("host" to uri.host.orEmpty(), "main" to request.isForMainFrame.toString()))
            // Only a page the student opened goes to the browser; other frames stay blocked.
            if (request.isForMainFrame) {
                try {
                    view.context.startActivity(Intent(Intent.ACTION_VIEW, uri).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
                } catch (_: ActivityNotFoundException) {
                    showToast("Can't open link", "No app on this phone opens it.", tone = ToastTone.Error)
                }
            }
            return true
        }

        override fun onPageStarted(view: WebView, url: String, favicon: Bitmap?) {
            pageStart = SystemClock.elapsedRealtime()
            pageLoading = true
            requests = 0
            progress = 0
            error = null
            innerAtTop = true
            if (isVtop(Uri.parse(url))) inject(view)
        }

        override fun doUpdateVisitedHistory(view: WebView, url: String, isReload: Boolean) = stepBack()

        override fun onPageCommitVisible(view: WebView, url: String) {
            hasContent = true
            if (isVtop(Uri.parse(url))) inject(view)
        }

        override fun onPageFinished(view: WebView, url: String) {
            pageLoading = false
            reloading = false
            hasContent = true
            val uri = Uri.parse(url)
            if (!isVtop(uri)) return
            val path = uri.path.orEmpty()
            if (path.startsWith("/vtop/login")) {
                Log.info(SRC, "login page", flow, mapOf("path" to path))
                return onLoginPage()
            }
            Log.info(SRC, "page loaded", flow, mapOf("path" to path, "ms" to (SystemClock.elapsedRealtime() - pageStart).toString()))
            inject(view)
            view.evaluateJavascript(VtopScripts.AUTHENTICATED) { signedIn -> if (signedIn == "true") onAuthenticated() }
        }

        override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
            Log.warn(SRC, "load failed", flow, mapOf("path" to pathOf(request.url), "code" to error.errorCode.toString(), "main" to request.isForMainFrame.toString()))
            if (request.isForMainFrame) {
                pageLoading = false
                reloading = false
                this@VtopBrowser.error = "VTOP could not load. Check your connection and try Home."
            }
        }

        override fun onReceivedHttpError(view: WebView, request: WebResourceRequest, response: WebResourceResponse) {
            Log.warn(SRC, "http error", flow, mapOf("path" to pathOf(request.url), "status" to response.statusCode.toString(), "main" to request.isForMainFrame.toString()))
            if (request.isForMainFrame) error = "VTOP returned an error. Try Home again."
        }

        // Handling this keeps Android from killing the whole app with the page's process.
        override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
            Log.warn(SRC, "render process gone", flow, mapOf("crash" to detail.didCrash().toString()))
            onLost()
            return true
        }
    }

    companion object {
        const val SRC = "ui.vtop"

        fun isVtop(uri: Uri) = uri.scheme == "https" && uri.host == Uri.parse(VtopScripts.ORIGIN).host

        /** A URL for logs: its path only, never the query (VTOP puts tokens there). */
        fun pathOf(url: String): String = Uri.parse(url).path.orEmpty()
        fun pathOf(uri: Uri): String = uri.path.orEmpty()
    }
}

/** Starts the jar over with [session]'s cookies, as the Flutter app does: VTOP's own host-only JSESSIONID would shadow them. */
internal suspend fun setCookies(session: WebSession) {
    val jar = CookieManager.getInstance()
    suspendCancellableCoroutine { done -> jar.removeAllCookies { done.resume(Unit) } }
    val url = session.baseUrl + "/vtop/"
    for (cookie in session.cookies) {
        val value = "${cookie.name}=${cookie.value}; Domain=${cookie.domain}; Path=${cookie.path}" + if (cookie.secure) "; Secure" else ""
        val set = suspendCancellableCoroutine { done -> jar.setCookie(url, value) { done.resume(it) } }
        check(set) { "Could not set VTOP cookies" }
    }
    jar.flush()
}
