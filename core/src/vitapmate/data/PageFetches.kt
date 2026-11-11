package vitapmate.data

import androidx.compose.runtime.mutableStateListOf
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.async
import vitapmate.observability.Log

/** A waiting fetch nobody needed any more, dropped before it ran ([SingleFlight]). */
internal class Dropped : Exception("dropped while waiting")

/**
 * One run of [work] per key at a time, in [scope] rather than the caller's: a caller that goes
 * away doesn't cancel it, and a caller that comes back for the same key gets the running (or
 * waiting) one, or learns that one just succeeded ([succeededWithin]; times from [clock], ms).
 *
 * At most [Tunable.FetchLanes] runs go at once, 1 on a slow link ([slowLink]). The rest wait,
 * and when a lane frees the next is picked then: pages on screen ([shown]) first, then pages the
 * student left, then background work; within one, runs the student asked for, then the oldest.
 * A waiting run is never cancelled by navigation, only moved; one that is automatic, has a cached
 * copy, is off screen and has waited over [Tunable.QueueDrop] is dropped instead (it fails with
 * [Dropped]). A running one always finishes. Queue events go to [log] (src `fetch.queue`).
 */
open class SingleFlight internal constructor(
    private val scope: CoroutineScope,
    private val log: (msg: String, flow: String, fields: Map<String, String>) -> Unit = { m, f, x -> Log.info("fetch.queue", m, f, x) },
    private val clock: () -> Long = System::currentTimeMillis,
) {
    /**
     * [result] of the run for a key, started in [flow]; [joined] when it was already running or
     * waiting; [started] completes when it gets a lane (true) or is dropped (false).
     */
    internal class Run<T>(val result: Deferred<T>, val flow: String, val joined: Boolean, val started: Deferred<Boolean>)

    private class Entry(
        val key: String,
        val result: Deferred<Any?>,
        val go: CompletableDeferred<Boolean>,
        val flow: String,
        val page: String,
        var user: Boolean,
        val hasCache: Boolean,
        val background: Boolean,
        val at: Long,
        val order: Long,
    ) {
        var queued = false
    }

    private val runs = mutableMapOf<String, Entry>()
    private var order = 0L

    /** When each key's last run succeeded. */
    private val succeeded = mutableMapOf<String, Long>()

    /** Page ids on screen, with how many screens show each. */
    private val onScreen = mutableMapOf<String, Int>()

    /** The last few page fetches' times (ms), and whether the last one came over UDP. */
    private val took = ArrayDeque<Long>()
    private var udp = false
    private var lanes = 0

    /** Page ids of the runs holding a lane, for "Waiting for Attendance to finish" (Compose state). */
    val runningPages = mutableStateListOf<String>()

    /**
     * The running or waiting run of [key], else [work] queued now in [flow]. [user]: the student
     * asked (goes first within its priority, never dropped); [hasCache]: the page has a copy to
     * keep showing if this one is dropped; [background]: the open-app sweep (lowest priority).
     */
    @Suppress("UNCHECKED_CAST")
    internal fun <T> start(
        key: String,
        flow: String,
        user: Boolean = true,
        hasCache: Boolean = false,
        background: Boolean = false,
        work: suspend () -> T,
    ): Run<T> = synchronized(runs) {
        runs[key]?.takeIf { it.result.isActive }?.let { e ->
            e.user = e.user || user
            return Run(e.result as Deferred<T>, e.flow, joined = true, e.go)
        }
        val go = CompletableDeferred<Boolean>()
        val result = scope.async {
            if (!go.await()) throw Dropped()
            work().also { synchronized(runs) { succeeded[key] = clock() } }
        }
        val entry = Entry(key, result, go, flow, key.substringBefore('/'), user, hasCache, background, clock(), order++)
        runs[key] = entry
        result.invokeOnCompletion {
            synchronized(runs) {
                if (runs[key] === entry) runs.remove(key)
                pump(entry.flow)
            }
        }
        pump(flow)
        if (!go.isCompleted) {
            entry.queued = true
            log("queued", flow, mapOf("page" to entry.page, "position" to "${waiting().indexOf(entry) + 1}", "priority" to priorityName(entry), "user" to "$user", "lanes" to "$lanes"))
        }
        Run(result, flow, joined = false, go)
    }

    /** The student asked for [key]'s run too (joined it): it goes ahead and is never dropped. */
    internal fun ask(key: String) = synchronized(runs) { runs[key]?.user = true }

    /** Whether a run of [key] is going on or waiting for a lane. */
    internal fun isRunning(key: String): Boolean = synchronized(runs) { runs[key]?.result?.isActive == true }

    /** Whether [key]'s last run succeeded at most [ms] ago. */
    internal fun succeededWithin(key: String, ms: Long): Boolean =
        synchronized(runs) { succeeded[key]?.let { clock() - it <= ms } ?: false }

    /** A screen showing page [id] came ([on]) or went; its waiting runs move up or down. */
    internal fun shown(id: String, on: Boolean) = synchronized(runs) {
        onScreen[id] = (onScreen[id] ?: 0) + if (on) 1 else -1
        if (onScreen[id]!! <= 0) onScreen.remove(id)
    }

    /** Whether a screen shows page [id]. */
    internal fun isShown(id: String): Boolean = synchronized(runs) { id in onScreen }

    /** A page fetch in [flow] took [ms] and came over UDP or not: sets the lanes ([slowLink]). */
    internal fun fetched(flow: String, ms: Long, udp: Boolean) = synchronized(runs) {
        took.addLast(ms)
        while (took.size > RECENT) took.removeFirst()
        this.udp = udp
        pump(flow)
    }

    /** The link is slow: most of the last few fetches took over [Tunable.SlowFetch], or it is UDP. */
    internal val slowLink: Boolean get() = synchronized(runs) { slow() }

    private fun slow() = udp || (took.sorted().getOrNull((took.size - 1) / 2) ?: 0) > Settings.tunable(Tunable.SlowFetch) * 1000L

    /** Cancels every run, waiting ones too, and forgets which succeeded (sign-out). */
    fun cancelAll() = synchronized(runs) {
        runs.values.forEach {
            it.go.cancel()
            it.result.cancel()
        }
        runs.clear()
        succeeded.clear()
        runningPages.clear()
    }

    private fun waiting() = runs.values.filter { !it.go.isCompleted }.sortedWith(compareBy({ rank(it) }, { !it.user }, { it.order }))

    private fun rank(e: Entry) = when {
        e.background -> 2
        e.page in onScreen -> 0
        else -> 1
    }

    private fun priorityName(e: Entry) = listOf("on_screen", "left", "background")[rank(e)]

    /** Drops stale waiting runs, then starts the next while a lane is free. Holds the lock. */
    private fun pump(flow: String) {
        val now = clock()
        val want = if (slow()) 1 else Settings.tunable(Tunable.FetchLanes)
        if (want != lanes) {
            if (lanes != 0) log("lanes", flow, mapOf("from" to "$lanes", "to" to "$want", "udp" to "$udp", "recent_ms" to took.joinToString(",")))
            lanes = want
        }
        val limit = Settings.tunable(Tunable.QueueDrop) * 1000L
        for (e in waiting()) {
            if (e.user || !e.hasCache || e.page in onScreen || now - e.at <= limit) continue
            runs.remove(e.key)
            e.go.complete(false)
            log("dropped", e.flow, mapOf("page" to e.page, "waited_ms" to "${now - e.at}"))
        }
        while (runs.values.count { it.go.isCompleted } < lanes) {
            val next = waiting().firstOrNull() ?: break
            next.go.complete(true)
            if (next.queued) log("started", next.flow, mapOf("page" to next.page, "waited_ms" to "${now - next.at}", "priority" to priorityName(next)))
        }
        val pages = runs.values.filter { it.go.isCompleted }.map { it.page }
        if (pages != runningPages) {
            runningPages.clear()
            runningPages.addAll(pages)
        }
    }

    private companion object {
        /** How many recent fetch times decide [slowLink]. */
        const val RECENT = 3
    }
}

/**
 * Page fetches ([rememberPage]) and the open-app sweep's ([StaleSweep]) by page and semester, for
 * the app's life: switching tabs mid-fetch no longer drops Rust's call (a re-login cut halfway,
 * the page never saved), and the page opened again joins the fetch and shows its result, or, when
 * it just finished, its saved copy without a second request. They share a few lanes, the page on
 * screen first, so on a slow link the page the student waits for isn't split with others. Only
 * sign-out cancels them.
 */
object PageFetches : SingleFlight(MainScope())
