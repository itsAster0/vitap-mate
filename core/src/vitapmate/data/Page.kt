package vitapmate.data

import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.OtpChallenge
import vitapmate.core.OtpListener
import vitapmate.core.PageId
import vitapmate.core.PageListener
import vitapmate.core.PageStatus
import vitapmate.core.PageWatch
import vitapmate.core.pageIdName
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.core.PageActivity as WatchActivity

/**
 * A VTOP page on a screen ([rememberPage]). Rust's watch decides when to fetch and tells each
 * status ([PageStatus]); [state] is that status with the data, read here with the screen's
 * `cached_x` call whenever the status names a new `updatedAt`. Rust logs every change as
 * `ui.<id>`; this reports it to [StateRegistry] as `page.<id>`. State changes on the main thread.
 */
@Stable
class PageHandle<T> internal constructor(
    val id: String,
    private val scope: CoroutineScope,
    private val cached: (core: Core, flow: String) -> T?,
) : PageListener {
    var state: PageState<T> by mutableStateOf(PageState.Loading())
        private set

    private val main by lazy { Handler(Looper.getMainLooper()) }
    private val watch = CompletableDeferred<PageWatch>()

    // Main thread only.
    private var status: PageStatus = PageStatus.Loading(null)
    private var data: T? = null
    private var dataAt: Long? = null
    private var reading = false
    private var closed = false
    private var current: PageWatch? = null
    private var lastProblem: Problem? = null

    override fun onStatus(status: PageStatus) {
        main.post { show(status) }
    }

    /**
     * Fetches fresh data in a flow `<id>.refresh`; [trigger] says why (`pull`, `button`, `retry`,
     * `otp`, ...); [user] false for automatic ones (their failure is [Problem.userInitiated]
     * false: no toast by default). Never throws (but cancellation): a failure goes into
     * [PageActivity.Stale], or [PageState.Failed] with nothing cached. A refresh while one runs
     * joins it.
     */
    suspend fun refresh(trigger: String, user: Boolean = true) {
        // A failure is in the state and the log already.
        attempt { flow("$id.refresh") { f -> watch.await().refresh(f, trigger, user) } }
    }

    internal fun opened(opened: PageWatch) = main.post {
        if (closed) {
            opened.stop()
        } else {
            opened.shown(true)
            current = opened
            watch.complete(opened)
        }
    }

    internal fun close() {
        closed = true
        current?.stop()
        report(onScreen = false)
    }

    private fun show(next: PageStatus) {
        if (closed) return
        status = next
        if (next is PageStatus.Ready && next.updatedAt != dataAt) read()
        state = stateOf(next)
        report(onScreen = true)
        val problem = problemIn(next)
        // No pick (any more): SemesterChoice.selected turns null and the app shows the picker.
        if (problem != lastProblem && problem?.kind == ProblemKind.SEMESTER_NEEDED) scope.launch { SemesterChoice.readCache(problem.flow) }
        lastProblem = problem
    }

    /** Reads the data the status names, again while it moves on meanwhile. */
    private fun read() {
        if (reading) return
        reading = true
        scope.launch {
            try {
                while (true) {
                    val want = status as? PageStatus.Ready ?: break
                    if (want.updatedAt == dataAt) break
                    val got = attempt { withContext(Dispatchers.IO) { cached(AppCore.await(), want.flow) } }.getOrElse { e ->
                        Log.warn("ui.$id", "cache unreadable, ignored: ${e::class.simpleName}", want.flow, mapOf("error" to e.toString().take(200)))
                        null
                    } ?: break
                    data = got
                    dataAt = want.updatedAt
                    state = stateOf(status)
                    report(onScreen = !closed)
                    // A fetch caches the semester list when there was none.
                    if (SemesterChoice.semesters == null) SemesterChoice.readCache(want.flow)
                }
            } finally {
                reading = false
            }
        }
    }

    private fun stateOf(status: PageStatus): PageState<T> = when (status) {
        is PageStatus.Loading -> PageState.Loading(status.waiting)
        is PageStatus.Failed -> PageState.Failed(status.problem)
        is PageStatus.Ready -> {
            val shown = data
            val at = dataAt
            if (shown != null && at != null) PageState.Ready(shown, at, activityOf(status.activity)) else state
        }
    }

    internal fun report(onScreen: Boolean) {
        val s = state
        val ready = status as? PageStatus.Ready
        StateRegistry.report(
            "page.$id",
            mapOf(
                "kind" to s.kindName(),
                "updatedAt" to (s as? PageState.Ready)?.updatedAt,
                "refreshing" to ((s as? PageState.Ready)?.activity is PageActivity.Refreshing),
                "fetching" to s.fetching,
                "waiting" to (((s as? PageState.Ready)?.activity as? PageActivity.Refreshing)?.waiting ?: (s as? PageState.Loading)?.waiting),
                "count" to ready?.count,
                "problem" to s.problem()?.kind?.name,
                "problemFlow" to s.problem()?.flow,
                "onScreen" to onScreen,
            ),
        )
    }
}

private fun activityOf(activity: WatchActivity): PageActivity = when (activity) {
    is WatchActivity.Idle -> PageActivity.Idle
    is WatchActivity.Refreshing -> PageActivity.Refreshing(activity.waiting)
    is WatchActivity.Stale -> PageActivity.Stale(activity.problem)
}

private fun problemIn(status: PageStatus): Problem? = when (status) {
    is PageStatus.Failed -> status.problem
    is PageStatus.Ready -> (status.activity as? WatchActivity.Stale)?.problem
    is PageStatus.Loading -> null
}

private fun PageState<*>.kindName() = when (this) {
    is PageState.Loading -> "Loading"
    is PageState.Ready -> "Ready"
    is PageState.Failed -> "Failed"
}

private fun PageState<*>.problem() = when (this) {
    is PageState.Ready -> (activity as? PageActivity.Stale)?.problem
    is PageState.Failed -> problem
    is PageState.Loading -> null
}

/** Whether a fetch of the page runs: [PageState.Loading], or a ready page refreshing. */
internal val PageState<*>.fetching: Boolean
    get() = this is PageState.Loading || (this as? PageState.Ready)?.activity is PageActivity.Refreshing

/**
 * The page [page] (its id is the log src `ui.<id>` and flows `<id>.open` / `<id>.refresh`): Rust
 * reads the cached copy first, then fetches when nothing is saved; with Auto Refresh on
 * ([Settings.autoRefresh]) also when the page was not fetched yet in this process, and when the
 * data is older than [Tunable.PageMaxAge] (6 h; on open or when the app comes back). Fetches
 * outlive the screen: one shown again while its fetch runs or waits joins it, or, when it just
 * finished, shows its saved copy with no second request. [item] tells apart pages of one id (a
 * course's history: its key; see [PageId]). [cached] gets the flow id to pass as the first
 * argument of the `Core` call, e.g. `rememberPage(PageId.ATTENDANCE, { c, f -> c.cachedAttendance(f) })`.
 */
@Composable
fun <T> rememberPage(page: PageId, cached: (core: Core, flow: String) -> T?, item: String = ""): PageHandle<T> {
    val scope = rememberCoroutineScope()
    // A new watch per semester: picking another one reads that semester's cache and fetches it.
    val semester = SemesterChoice.selected
    // The handle outlives recompositions: it calls the latest lambda, not the first one's captures.
    val latestCached by rememberUpdatedState(cached)
    val handle = remember(page, item, semester) { PageHandle(pageIdName(page), scope) { c, f -> latestCached(c, f) } }
    DisposableEffect(handle) {
        // Not cancelled with the screen: a watch opened late is closed at once ([PageHandle.opened]).
        AppScope.launch {
            attempt {
                flow("${handle.id}.open") { f ->
                    val core = AppCore.await()
                    handle.opened(withContext(Dispatchers.IO) { core.pageWatch(f, page, item, handle) })
                }
            }.onFailure { Log.warn("ui.${handle.id}", "not watched: ${it::class.simpleName}") }
        }
        onDispose { handle.close() }
    }
    return handle
}

/**
 * The OTP prompt Rust opens when a sign-in the student watches wants the emailed code (Rust's
 * `otp.rs`): the calls wait there for its outcome, so nothing here retries. The login feature's
 * sheet shows [challenge] while it is set; a code accepted anywhere (the sheet, auto-fill,
 * another route) closes it from Rust, and [cancel] closes it for the student. Told on Rust's
 * threads, read anywhere.
 */
object OtpPrompt : OtpListener {
    private val shown = MutableStateFlow<OtpChallenge?>(null)

    /** The challenge the sheet shows; null while none is open. */
    val challenge: StateFlow<OtpChallenge?> = shown.asStateFlow()

    /** Rust's prompt, and the one the student closed (hidden at once, before Rust hears). */
    private var told: OtpChallenge? = null
    private var dismissed: ULong? = null

    override fun onChallenge(challenge: OtpChallenge?) = synchronized(this) {
        told = challenge
        publish()
    }

    private fun publish() {
        shown.value = told?.takeIf { it.id != dismissed }
    }

    /** Whether a prompt is open. */
    val pending: Boolean get() = challenge.value != null

    /**
     * Waits for the code of the sign-in that asked for it (the Account page's `NeedsOtp`, Fetch
     * all's): opens the prompt; true once VTOP accepted the code, false when the student closed
     * it or it failed otherwise (in the log).
     */
    suspend fun await(flow: String): Boolean = attempt { withContext(Dispatchers.IO) { AppCore.await().awaitOtp(flow) } }.isSuccess

    /**
     * The student dismissed the prompt: no fetch again (pull or the refresh button retries). Rust
     * drops the pending challenge, so that retry signs in afresh and VTOP mails a new code.
     */
    fun cancel() {
        synchronized(this) {
            dismissed = told?.id
            publish()
        }
        // Outlives the prompt, which closes as soon as it is cancelled.
        AppScope.launch {
            // A failure (signed out meanwhile, or nothing pending) is in the log already.
            attempt { coreFlow("otp.cancel", Core::cancelOtp) }
        }
    }
}
