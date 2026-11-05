package vitapmate.data

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.async

/**
 * One run of [work] per key at a time, in [scope] rather than the caller's: a caller that goes
 * away doesn't cancel it, and a caller that comes back for the same key gets the running one, or
 * learns that one just succeeded ([succeededWithin]; times from [clock], ms).
 */
open class SingleFlight internal constructor(
    private val scope: CoroutineScope,
    private val clock: () -> Long = System::currentTimeMillis,
) {
    /** [result] of the run for a key, started in [flow]; [joined] when it was already running. */
    internal class Run<T>(val result: Deferred<T>, val flow: String, val joined: Boolean)

    private class Entry(val result: Deferred<Any?>, val flow: String)

    private val running = mutableMapOf<String, Entry>()

    /** When each key's last run succeeded. */
    private val succeeded = mutableMapOf<String, Long>()

    /** The running run of [key], else [work] started now in [flow]. */
    @Suppress("UNCHECKED_CAST")
    internal fun <T> start(key: String, flow: String, work: suspend () -> T): Run<T> = synchronized(running) {
        running[key]?.takeIf { it.result.isActive }?.let { return Run(it.result as Deferred<T>, it.flow, joined = true) }
        val result = scope.async { work().also { synchronized(running) { succeeded[key] = clock() } } }
        running[key] = Entry(result, flow)
        result.invokeOnCompletion { synchronized(running) { if (running[key]?.result === result) running.remove(key) } }
        Run(result, flow, joined = false)
    }

    /** Whether a run of [key] is going on. */
    internal fun isRunning(key: String): Boolean = synchronized(running) { running[key]?.result?.isActive == true }

    /** Whether [key]'s last run succeeded at most [ms] ago. */
    internal fun succeededWithin(key: String, ms: Long): Boolean =
        synchronized(running) { succeeded[key]?.let { clock() - it <= ms } ?: false }

    /** Cancels every run and forgets which succeeded (sign-out). */
    fun cancelAll() = synchronized(running) {
        running.values.forEach { it.result.cancel() }
        running.clear()
        succeeded.clear()
    }
}

/**
 * Page fetches ([rememberPage]) by page and semester, for the app's life: switching tabs mid-fetch
 * no longer drops Rust's call (a re-login cut halfway, the page never saved), and the page opened
 * again joins the fetch and shows its result, or, when it just finished, its saved copy without a
 * second request. Only sign-out cancels them.
 */
object PageFetches : SingleFlight(MainScope())
