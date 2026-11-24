package vitapmate.data

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.core.Route
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import kotlin.time.Duration.Companion.milliseconds

/**
 * A VTOP page on a screen ([rememberPage]). [state] is the cache, then the fetch's result; every
 * change is logged as `ui.<id>` (state, ms since open, count) and reported to [StateRegistry]
 * as `page.<id>`.
 */
@Stable
class PageHandle<T> internal constructor(
    val id: String,
    /** The page, its item and the semester: fetches with the same key are one ([PageFetches]). */
    internal val key: String,
    private val scope: CoroutineScope,
    private val cached: (core: Core, flow: String) -> T?,
    private val fetch: suspend (core: Core, flow: String) -> T,
    private val updatedAt: (T) -> Long,
    private val count: (T) -> Int?,
) {
    var state: PageState<T> by mutableStateOf(PageState.Loading())
        private set

    internal var fetching by mutableStateOf(false)
        private set
    private var running: Pair<Deferred<Throwable?>, Asked>? = null

    /** Whether the student asked for a running fetch (it started for them, or they joined it). */
    private class Asked(@Volatile var user: Boolean)
    private val opened = System.nanoTime()

    /**
     * Fetches fresh data in a flow `<id>.refresh`; [trigger] says why (`pull`, `button`, `retry`,
     * `open`, `stale`, `otp`, ...); [user] false for automatic ones (their failure is
     * [Problem.userInitiated] false: no toast by default). Never throws (but cancellation): a
     * failure goes into [PageActivity.Stale], or [PageState.Failed] with nothing cached. A
     * refresh while one runs waits for that one instead of fetching twice.
     */
    suspend fun refresh(trigger: String, user: Boolean = true) {
        // A failure is in the state and the log already.
        attempt { flow("$id.refresh") { f -> fetchIn(f, trigger, user)?.let { throw it } } }
    }

    /** One fetch at a time: joins the running one, else starts one; returns its failure. */
    internal suspend fun fetchIn(flow: String, trigger: String, user: Boolean = true): Throwable? {
        val (job, joined) = synchronized(this) {
            running?.takeIf { it.first.isActive }?.let { (job, asked) -> asked.user = asked.user || user; job to true }
                ?: Asked(user).let { asked -> scope.async { fetchNow(flow, trigger, asked) }.also { running = it to asked } }.let { it to false }
        }
        if (joined) {
            if (user) PageFetches.ask(key)
            log("join", flow, "trigger" to trigger)
        }
        return job.await()
    }

    private suspend fun fetchNow(flow: String, trigger: String, asked: Asked): Throwable? {
        fetching = true
        val before = state
        if (before is PageState.Ready) set(before.copy(activity = PageActivity.Refreshing()), "refreshing", flow, "trigger" to trigger)
        else set(PageState.Loading(), "fetching", flow, "trigger" to trigger)
        // The fetch itself runs in PageFetches: a tab switch doesn't cancel it, and a handle for the
        // same page and semester (the tab shown again) joins it. It may wait for a lane first (shown
        // as [PageState.Loading.waiting]). Problems name the flow that ran it.
        val run = PageFetches.start(key, flow, asked.user, hasCache = before is PageState.Ready) { download(flow) }
        if (run.joined) log("join", flow, "trigger" to trigger, "running" to run.flow)
        val note = scope.launch { noteWaiting(run.started) }
        return try {
            val fresh = run.result.await()
            set(PageState.Ready(fresh, updatedAt(fresh)), "fresh", flow)
            null
        } catch (e: CancellationException) {
            (state as? PageState.Ready)?.let { state = it.copy(activity = it.activity.settled()) }
            throw e
        } catch (_: Dropped) {
            // Waited off screen past Tunable.QueueDrop: the cached copy stays, no problem.
            (state as? PageState.Ready)?.let { set(it.copy(activity = it.activity.settled()), "dropped", flow) }
            null
        } catch (e: Throwable) {
            val problem = problemOf(e, run.flow).copy(userInitiated = asked.user)
            val now = state
            if (now is PageState.Ready) set(now.copy(activity = PageActivity.Stale(problem)), "stale_kept", flow, "error" to e.kindName(), "user" to "${asked.user}")
            else set(PageState.Failed(problem), "failed", flow, "error" to e.kindName(), "user" to "${asked.user}")
            e
        } finally {
            note.cancel()
            fetching = false
        }
    }

    /** Still waiting for a lane after a moment: says which pages hold them until [started]. */
    private suspend fun noteWaiting(started: Deferred<Boolean>) {
        delay(300.milliseconds)
        if (started.isCompleted) return
        try {
            coroutineScope {
                val names = launch { snapshotFlow { PageFetches.runningPages.toList() }.collect { waiting(waitingFor(it)) } }
                started.join()
                names.cancel()
            }
        } finally {
            waiting(null)
        }
    }

    private fun waiting(text: String?) {
        state = when (val s = state) {
            is PageState.Loading -> s.copy(waiting = text)
            is PageState.Ready -> s.copy(activity = s.activity.waiting(text))
            is PageState.Failed -> s
        }
    }

    /** One fetch of the page, with what follows from it once (however many handles wait for it). */
    private suspend fun download(flow: String, retried: Boolean = false): T {
        val start = System.nanoTime()
        val otpRound = OtpPrompt.generation
        val otpAsks = OtpPrompt.asks
        return attempt {
            var udp = false
            val fresh = Activity.track("Refreshing ${id.replace('_', ' ')}", flow) {
                withContext(Dispatchers.IO) {
                    val core = AppCore.await()
                    fetch(core, flow).also { udp = core.routeStatus(flow).lastUsed == Route.UDP }
                }
            }
            val ms = (System.nanoTime() - start) / 1_000_000
            PageFetches.fetched(flow, ms, udp)
            StaleSweep.afterFetch(ms)
            // A fetch caches the semester list when there was none.
            if (SemesterChoice.semesters == null) SemesterChoice.readCache(flow)
            OtpPrompt.signedIn(otpAsks, flow)
            fresh
        }.getOrElse { e ->
            // A code was accepted while this fetch ran: VTOP is signed in now, so fetch again
            // instead of asking for another code.
            if (OtpPrompt.generation != otpRound) {
                if (e is CoreException.OtpRequired && !retried) return download(flow, retried = true)
            } else askedOtp(e)
            // No pick (any more): SemesterChoice.selected turns null and the app shows the picker.
            if (e is CoreException.SemesterNeeded) SemesterChoice.readCache(flow)
            throw e
        }
    }

    /** Shows the cached copy; a corrupt or unreadable cache counts as none. */
    internal suspend fun readCache(flow: String) {
        if (state is PageState.Ready) return
        val data = attempt {
            withContext(Dispatchers.IO) { cached(AppCore.await(), flow) }
        }.getOrElse { e ->
            Log.warn("ui.$id", "cache unreadable, ignored: ${e.kindName()}", flow, mapOf("error" to e.toString().take(200)))
            null
        }
        if (data == null) return log("no_cache", flow)
        when (val s = state) {
            is PageState.Ready -> return // a fetch finished first
            // A fetch failed first (e.g. offline): the cache, with its problem.
            else -> set(PageState.Ready(data, updatedAt(data), PageActivity.cached(s, fetching)), "cached", flow)
        }
    }

    /** Ready data older than [Tunable.PageMaxAge] (6 h) is fetched again on open and on return. */
    internal fun isStale(nowSeconds: Long = System.currentTimeMillis() / 1000): Boolean =
        (state as? PageState.Ready)?.let { nowSeconds - it.updatedAt > Settings.tunable(Tunable.PageMaxAge) * 3600L } ?: false

    private fun set(new: PageState<T>, what: String, flow: String, vararg extra: Pair<String, String>) {
        state = new
        log(what, flow, *extra)
    }

    private fun log(what: String, flow: String, vararg extra: Pair<String, String>) {
        val s = state
        val fields = buildMap {
            put("state", what)
            put("kind", s.kindName())
            put("ms", ((System.nanoTime() - opened) / 1_000_000).toString())
            (s as? PageState.Ready)?.let { r ->
                count(r.data)?.let { put("count", "$it") }
                put("age_s", "${System.currentTimeMillis() / 1000 - r.updatedAt}")
            }
            s.problem()?.let { put("problem", it.kind.name) }
            putAll(extra)
        }
        Log.info("ui.$id", what, flow, fields)
        report(onScreen = true)
    }

    internal fun report(onScreen: Boolean) {
        val s = state
        StateRegistry.report(
            "page.$id",
            mapOf(
                "kind" to s.kindName(),
                "updatedAt" to (s as? PageState.Ready)?.updatedAt,
                "refreshing" to ((s as? PageState.Ready)?.activity is PageActivity.Refreshing),
                "waiting" to (((s as? PageState.Ready)?.activity as? PageActivity.Refreshing)?.waiting ?: (s as? PageState.Loading)?.waiting),
                "count" to (s as? PageState.Ready)?.let { count(it.data) },
                "problem" to s.problem()?.kind?.name,
                "problemFlow" to s.problem()?.flow,
                "onScreen" to onScreen,
            ),
        )
    }
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

private fun Throwable.kindName() = this::class.simpleName ?: "Throwable"

/** "Waiting for Attendance to finish" behind the page ids holding the lanes; null with none. */
private fun waitingFor(pages: List<String>) = when (pages.size) {
    0 -> null
    1 -> "Waiting for ${pages[0].replace('_', ' ').replaceFirstChar { it.uppercase() }} to finish"
    else -> "Waiting for other pages to finish"
}

/**
 * The page [id] (also its log src `ui.<id>` and flows `<id>.open` / `<id>.refresh`): the cached
 * copy first (read off the main thread), then a fetch when nothing is saved; with Auto Refresh on
 * ([Settings.autoRefresh]) also when the page was not fetched yet in this screen's saved state,
 * when the data is older than [Tunable.PageMaxAge] (6 h; on open or when the app comes back), and after the OTP prompt
 * is answered. [item] tells apart pages of one [id] (a course's history: its key). Fetches outlive
 * the screen ([PageFetches]): one shown again while its fetch runs or waits joins it. They share a
 * few lanes, this page first while it is on screen; queued a moment, the state says which page it
 * waits for. An automatic fetch left waiting off screen too long is dropped (the cache stays).
 * The lambdas get the flow id to pass as the first argument of the `Core` call, e.g.
 * `rememberPage("attendance", { c, f -> c.cachedAttendance(f) }, { c, f -> c.fetchAttendance(f) }, { it.updatedAt }) { it.courses.size }`.
 */
@Composable
fun <T> rememberPage(
    id: String,
    cached: (core: Core, flow: String) -> T?,
    fetch: suspend (core: Core, flow: String) -> T,
    updatedAt: (T) -> Long,
    item: String = "",
    count: (T) -> Int? = { null },
): PageHandle<T> {
    val scope = rememberCoroutineScope()
    // A new handle per semester: picking another one reads that semester's cache and fetches it.
    val semester = SemesterChoice.selected
    // The handle outlives recompositions: it calls the latest lambdas, not the first one's captures.
    val latestCached by rememberUpdatedState(cached)
    val latestFetch by rememberUpdatedState(fetch)
    val page = remember(id, item, semester) {
        PageHandle(id, "$id/$item/$semester", scope, { c, f -> latestCached(c, f) }, { c, f -> latestFetch(c, f) }, updatedAt, count)
    }
    // Which semester this screen fetched, saved by value: a key alone can come back restored for
    // another semester when the tab returns to the screen.
    var fetchedFor by rememberSaveable(id) { mutableStateOf<String?>(null) }
    val otpAtStart = remember { OtpPrompt.generation }
    LaunchedEffect(page) {
        flow("$id.open") { f -> page.readCache(f) }
        // Its fetch finished while the screen was away (a tab switch mid-fetch, at most
        // [Tunable.JustFetched] ago): the cache just read is that result, so no second request.
        if (fetchedFor != semester && page.state is PageState.Ready && PageFetches.succeededWithin(page.key, Settings.tunable(Tunable.JustFetched) * 60_000L)) {
            fetchedFor = semester
        }
        val ready = page.state is PageState.Ready
        val trigger = refreshTrigger(
            PageWake.Open,
            ready = ready,
            running = ready && PageFetches.isRunning(page.key),
            autoRefresh = Settings.autoRefresh,
            fetched = fetchedFor == semester,
            stale = page.isStale(),
        ) ?: return@LaunchedEffect
        page.refresh(trigger, user = false)
        if (page.state.let { it is PageState.Ready && it.activity !is PageActivity.Stale }) fetchedFor = semester
    }
    LaunchedEffect(page, OtpPrompt.generation) {
        if (OtpPrompt.generation != otpAtStart) page.refresh(refreshTrigger(PageWake.Otp) ?: return@LaunchedEffect)
    }
    // Back in the app after a while: old data is fetched again.
    val resumesAtStart = remember { Foreground.resumes }
    LaunchedEffect(page, Foreground.resumes) {
        if (Foreground.resumes == resumesAtStart) return@LaunchedEffect
        val trigger = refreshTrigger(PageWake.Resume, autoRefresh = Settings.autoRefresh, stale = page.isStale())
        page.refresh(trigger ?: return@LaunchedEffect, user = false)
    }
    DisposableEffect(page) {
        PageFetches.shown(id, true)
        onDispose {
            PageFetches.shown(id, false)
            page.report(onScreen = false)
        }
    }
    return page
}

/** What wakes a page's refresh effect: the screen opened, the OTP prompt was answered, the app came back. */
internal enum class PageWake { Open, Otp, Resume }

/**
 * Why [rememberPage] fetches on [wake], or null for no fetch. [ready]: something is shown;
 * [running]: its fetch still runs (the screen left mid-fetch); [fetched]: this screen fetched the
 * selected semester already; [stale]: older than [Tunable.PageMaxAge].
 */
internal fun refreshTrigger(
    wake: PageWake,
    ready: Boolean = true,
    running: Boolean = false,
    autoRefresh: Boolean = true,
    fetched: Boolean = true,
    stale: Boolean = false,
): String? = when (wake) {
    PageWake.Otp -> "otp"
    PageWake.Resume -> "resume".takeIf { autoRefresh && stale }
    PageWake.Open -> when {
        !ready -> "open" // nothing to show: always fetch
        // Join it (no request) to show its result.
        running -> "rejoin"
        !autoRefresh -> null
        !fetched -> "open"
        stale -> "stale"
        else -> null
    }
}

/**
 * Set when a fetch's re-login wants the security OTP. The login feature shows the prompt while
 * [message] is set, submits the code, then calls [done] (open pages fetch again) or [cancel].
 */
object OtpPrompt {
    var message by mutableStateOf<String?>(null)
        private set
    var generation by mutableIntStateOf(0)
        private set

    /** Monotonic id per distinct prompt open, so consecutive prompts with the same message don't collide. */
    var challengeId by mutableIntStateOf(0)
        private set

    /** Times the prompt was asked for, so a fetch can tell it opened before the fetch began. */
    var asks = 0
        private set

    fun ask(message: String) {
        asks++
        if (this.message == null) challengeId++
        this.message = message
    }

    /**
     * A live fetch got through while the prompt was open since before it started ([asksAtStart]):
     * VTOP got signed in another way (another route, the server, background auto-fill), so the
     * prompt closes as if the code was accepted and open pages fetch again.
     */
    fun signedIn(asksAtStart: Int, flow: String) {
        if (message == null || asks != asksAtStart) return
        Log.info("ui.otp", "signed in meanwhile, prompt closed", flow)
        done()
    }

    /** [ask]s, then waits for the prompt to close; true when the code was accepted ([done]). */
    suspend fun askAndWait(message: String): Boolean {
        val before = generation
        ask(message)
        snapshotFlow { this.message }.first { it == null }
        return generation != before
    }

    /** The code was accepted: open pages fetch again. */
    fun done() {
        message = null
        generation++
    }

    /**
     * The student dismissed the prompt: no fetch again (pull or the refresh button retries). Rust
     * drops the pending challenge, so that retry signs in afresh and VTOP mails a new code.
     */
    fun cancel() {
        message = null
        // Outlives the prompt, which closes as soon as it is cancelled.
        AppScope.launch {
            // A failure (signed out meanwhile, or nothing pending) is in the log already.
            attempt { coreFlow("otp.cancel", Core::cancelOtp) }
        }
    }
}
