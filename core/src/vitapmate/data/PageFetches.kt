package vitapmate.data

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
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
 * [Dropped]). A running one always finishes. The decisions are [FetchQueue]'s; this runs them.
 * Queue events go to [log] (src `fetch.queue`).
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

    private class Handle(val go: CompletableDeferred<Boolean>, val result: Deferred<Any?>)

    private val lock = Any()
    private var queue = FetchQueue()

    /** The runs' coroutines by ticket id, until they end. */
    private val handles = mutableMapOf<Long, Handle>()

    private val running = MutableStateFlow<List<String>>(emptyList())

    /** Page ids of the runs holding a lane, for "Waiting for Attendance to finish". */
    val runningPages: StateFlow<List<String>> = running.asStateFlow()

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
    ): Run<T> {
        val (started, effects) = synchronized(lock) {
            queue.active(key)?.let { t ->
                val handle = handles.getValue(t.id)
                val effects = if (user && !t.user) step(QueueEvent.Ask(key)) else emptyList()
                return@synchronized Run(handle.result as Deferred<T>, t.flow, joined = true, handle.go) to effects
            }
            val ticket = Ticket(queue.nextId, key, flow, user, hasCache, background, clock())
            val go = CompletableDeferred<Boolean>()
            val result = scope.async {
                if (!go.await()) throw Dropped()
                work()
            }
            handles[ticket.id] = Handle(go, result)
            result.invokeOnCompletion { cause ->
                perform(
                    synchronized(lock) {
                        handles.remove(ticket.id)
                        step(QueueEvent.Finished(ticket.id, key, flow, succeeded = cause == null))
                    },
                )
            }
            Run(result, flow, joined = false, go) to step(QueueEvent.Start(ticket))
        }
        perform(effects)
        return started
    }

    /** The student asked for [key]'s run too (joined it): it goes ahead and is never dropped. */
    internal fun ask(key: String) = dispatch(QueueEvent.Ask(key))

    /** Whether a run of [key] is going on or waiting for a lane. */
    internal fun isRunning(key: String): Boolean = synchronized(lock) { queue.active(key) != null }

    /** Whether [key]'s last run succeeded at most [ms] ago. */
    internal fun succeededWithin(key: String, ms: Long): Boolean = synchronized(lock) { queue.succeededWithin(key, ms, clock()) }

    /** A screen showing page [id] came ([on]) or went; its waiting runs move up or down. */
    internal fun shown(id: String, on: Boolean) = dispatch(QueueEvent.Shown(id, on))

    /** Whether a screen shows page [id]. */
    internal fun isShown(id: String): Boolean = synchronized(lock) { queue.isShown(id) }

    /** A page fetch in [flow] took [ms] and came over UDP or not: sets the lanes ([slowLink]). */
    internal fun fetched(flow: String, ms: Long, udp: Boolean) = dispatch(QueueEvent.Fetched(flow, ms, udp))

    /** The link is slow: most of the last few fetches took over [Tunable.SlowFetch], or it is UDP. */
    internal val slowLink: Boolean get() = synchronized(lock) { queue.slow(QueueConfig.current()) }

    /** Cancels every run, waiting ones too, and forgets which succeeded (sign-out). */
    fun cancelAll() = dispatch(QueueEvent.CancelAll)

    private fun dispatch(event: QueueEvent) = perform(synchronized(lock) { step(event) })

    /** [event] applied to the queue; its effects with their runs, to [perform] outside the lock. Holds the lock. */
    private fun step(event: QueueEvent): List<Pair<QueueEffect, Handle?>> {
        val next = queue.next(event, clock(), QueueConfig.current())
        queue = next.queue
        running.value = queue.runningPages
        return next.effects.map { e ->
            e to when (e) {
                is QueueEffect.Go -> handles[e.ticketId]
                is QueueEffect.Drop -> handles.remove(e.ticketId)
                is QueueEffect.Cancel -> handles.remove(e.ticketId)
                is QueueEffect.Log -> null
            }
        }
    }

    /** Outside the lock: a run resumed here may start work that comes back to the queue. */
    private fun perform(effects: List<Pair<QueueEffect, Handle?>>) {
        for ((e, handle) in effects) when (e) {
            is QueueEffect.Go -> handle?.go?.complete(true)
            is QueueEffect.Drop -> handle?.go?.complete(false)
            is QueueEffect.Cancel -> handle?.let {
                it.go.cancel()
                it.result.cancel()
            }
            is QueueEffect.Log -> log(e.msg, e.flow, e.fields)
        }
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
object PageFetches : SingleFlight(AppScope)
