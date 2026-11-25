package vitapmate.data

import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.isActive
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.core.OtpChallenge
import vitapmate.core.OtpListener
import vitapmate.core.Route
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import kotlin.time.Duration.Companion.milliseconds

/**
 * A VTOP page on a screen ([rememberPage]). [state] is the cache, then the fetch's result, each
 * change a [PageEvent] ([next]); every change is logged as `ui.<id>` (state, ms since open,
 * count) and reported to [StateRegistry] as `page.<id>`. State changes on the main thread.
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

    /** Whether the student asked for a running fetch (it started for them, or they joined it). */
    private class Asked(@Volatile var user: Boolean)

    /** This handle's fetch while it runs. */
    private class Running(val job: Deferred<Throwable?>, val asked: Asked)
    private var running: Running? = null

    /** Done once the cache was read (or found missing): fetches start after it, so it never lands over theirs. */
    private val cacheRead = CompletableDeferred<Unit>()
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
        cacheRead.await()
        val (job, joined) = synchronized(this) {
            running?.takeIf { it.job.isActive }?.let { r ->
                r.asked.user = r.asked.user || user
                r.job to true
            } ?: Asked(user).let { asked ->
                val job = scope.async(start = CoroutineStart.LAZY) { fetchNow(flow, trigger, asked) }
                running = Running(job, asked)
                job to false
            }
        }
        if (joined) {
            if (user) PageFetches.ask(key)
            log("join", flow, "trigger" to trigger)
        } else {
            job.start()
        }
        return job.await()
    }

    private suspend fun fetchNow(flow: String, trigger: String, asked: Asked): Throwable? {
        val hasCache = state is PageState.Ready
        on(PageEvent.FetchStarted, if (hasCache) "refreshing" else "fetching", flow, "trigger" to trigger)
        // The fetch itself runs in PageFetches: a tab switch doesn't cancel it, and a handle for the
        // same page and semester (the tab shown again) joins it. It may wait for a lane first (shown
        // as [PageState.Loading.waiting]). Problems name the flow that ran it.
        val run = PageFetches.start(key, flow, asked.user, hasCache) { download(flow) }
        if (run.joined) log("join", flow, "trigger" to trigger, "running" to run.flow)
        val note = scope.launch { noteWaiting(run.started) }
        return try {
            val fresh = run.result.await()
            on(PageEvent.FetchSucceeded(fresh, updatedAt(fresh)), "fresh", flow)
            null
        } catch (e: CancellationException) {
            on(PageEvent.FetchCancelled(run.flow), "cancelled", flow)
            // Only the run was cancelled (sign-out): the state says so. This screen went away: stop.
            if (currentCoroutineContext().isActive) null else throw e
        } catch (_: Dropped) {
            // Waited off screen past Tunable.QueueDrop: the cached copy stays, no problem.
            on(PageEvent.FetchDropped(run.flow), "dropped", flow)
            null
        } catch (e: Throwable) {
            val problem = problemOf(e, run.flow).copy(userInitiated = asked.user)
            on(PageEvent.FetchFailed(problem), if (state is PageState.Ready) "stale_kept" else "failed", flow, "error" to e.kindName(), "user" to "${asked.user}")
            e
        } finally {
            note.cancel()
        }
    }

    /** Still waiting for a lane after a moment: says which pages hold them until [started]. */
    private suspend fun noteWaiting(started: Deferred<Boolean>) {
        delay(300.milliseconds)
        if (started.isCompleted) return
        try {
            coroutineScope {
                val names = launch { PageFetches.runningPages.collect { state = state.next(PageEvent.WaitingChanged(waitingFor(it))) } }
                started.join()
                names.cancel()
            }
        } finally {
            state = state.next(PageEvent.WaitingChanged(null))
        }
    }

    /**
     * One fetch of the page, with what follows from it once (however many handles wait for it).
     * A re-login that wants the OTP waits for it in Rust (the prompt opens), then goes on.
     */
    private suspend fun download(flow: String): T {
        val start = System.nanoTime()
        val prompts = OtpPrompt.mark()
        return attempt {
            var udp = false
            val fresh = Activity.track("Refreshing ${id.replace('_', ' ')}", flow) {
                withContext(Dispatchers.IO) {
                    val core = AppCore.await()
                    fetch(core, flow).also { udp = core.routeStatus(flow).lastUse?.route == Route.UDP }
                }
            }
            val ms = (System.nanoTime() - start) / 1_000_000
            // A fetch that waited for the student's code says nothing about the link.
            if (!OtpPrompt.openedSince(prompts)) {
                PageFetches.fetched(flow, ms, udp)
                StaleSweep.afterFetch(ms)
            }
            // A fetch caches the semester list when there was none.
            if (SemesterChoice.semesters == null) SemesterChoice.readCache(flow)
            fresh
        }.getOrElse { e ->
            // No pick (any more): SemesterChoice.selected turns null and the app shows the picker.
            if (e is CoreException.SemesterNeeded) SemesterChoice.readCache(flow)
            throw e
        }
    }

    /** Shows the cached copy, before any fetch; a corrupt or unreadable cache counts as none. */
    internal suspend fun readCache(flow: String) {
        try {
            if (state is PageState.Ready) return
            val data = attempt {
                withContext(Dispatchers.IO) { cached(AppCore.await(), flow) }
            }.getOrElse { e ->
                Log.warn("ui.$id", "cache unreadable, ignored: ${e.kindName()}", flow, mapOf("error" to e.toString().take(200)))
                null
            }
            if (data == null) return log("no_cache", flow)
            on(PageEvent.CacheLoaded(data, updatedAt(data)), "cached", flow)
        } finally {
            cacheRead.complete(Unit)
        }
    }

    /** Ready data older than [Tunable.PageMaxAge] (6 h) is fetched again on open and on return. */
    internal fun isStale(nowSeconds: Long = System.currentTimeMillis() / 1000): Boolean =
        (state as? PageState.Ready)?.let { nowSeconds - it.updatedAt > Settings.tunable(Tunable.PageMaxAge) * 3600L } ?: false

    private fun on(event: PageEvent<T>, what: String, flow: String, vararg extra: Pair<String, String>) {
        state = state.next(event)
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
                "fetching" to s.fetching,
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
 * and when the data is older than [Tunable.PageMaxAge] (6 h; on open or when the app comes back).
 * A fetch whose re-login wants the OTP waits for it ([OtpPrompt]) and then goes on. [item] tells
 * apart pages of one [id] (a course's history: its key). Fetches outlive the screen
 * ([PageFetches]): one shown again while its fetch runs or waits joins it. They share a few
 * lanes, this page first while it is on screen; queued a moment, the state says which page it
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
    val resumesAtStart = remember { Foreground.resumes }
    // One effect, in turn: the cache, the fetch on open, then a fetch each time the app comes
    // back with old data. None of them overlaps another (and fetches wait for the cache).
    LaunchedEffect(page) {
        flow("$id.open") { f -> page.readCache(f) }
        // Its fetch finished while the screen was away (a tab switch mid-fetch, at most
        // [Tunable.JustFetched] ago): the cache just read is that result, so no second request.
        if (fetchedFor != semester && page.state is PageState.Ready && PageFetches.succeededWithin(page.key, Settings.tunable(Tunable.JustFetched) * 60_000L)) {
            fetchedFor = semester
        }
        val ready = page.state is PageState.Ready
        refreshTrigger(
            PageWake.Open,
            ready = ready,
            running = ready && PageFetches.isRunning(page.key),
            autoRefresh = Settings.autoRefresh,
            fetched = fetchedFor == semester,
            stale = page.isStale(),
        )?.let { trigger ->
            page.refresh(trigger, user = false)
            if (page.state.let { it is PageState.Ready && it.activity !is PageActivity.Stale }) fetchedFor = semester
        }
        // Back in the app after a while: old data is fetched again.
        var seen = resumesAtStart
        snapshotFlow { Foreground.resumes }.collect { resumes ->
            if (resumes == seen) return@collect
            seen = resumes
            refreshTrigger(PageWake.Resume, autoRefresh = Settings.autoRefresh, stale = page.isStale())?.let { page.refresh(it, user = false) }
        }
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

/** What wakes a page's refresh effect: the screen opened, or the app came back. */
internal enum class PageWake { Open, Resume }

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

    /** Rust's prompt, the one the student closed (hidden at once, before Rust hears), and the newest id seen. */
    private var told: OtpChallenge? = null
    private var dismissed: ULong? = null
    private var newest: ULong = 0u

    override fun onChallenge(challenge: OtpChallenge?) = synchronized(this) {
        told = challenge
        challenge?.let { newest = maxOf(newest, it.id) }
        publish()
    }

    private fun publish() {
        shown.value = told?.takeIf { it.id != dismissed }
    }

    /** Whether a prompt is open. */
    val pending: Boolean get() = challenge.value != null

    /** A mark for [openedSince]: null while a prompt is open. */
    internal fun mark(): ULong? = synchronized(this) { newest.takeIf { told == null } }

    /** Whether a prompt was open at [mark] or opened since. */
    internal fun openedSince(mark: ULong?): Boolean = synchronized(this) { mark == null || newest != mark }

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
