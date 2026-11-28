package vitapmate.data

import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import vitapmate.core.QueueEnd
import vitapmate.core.QueuedWork

/** A waiting fetch nobody needed any more, dropped before it ran. */
internal class Dropped : Exception("dropped while waiting")

/**
 * Rust's fetch queue, for the runs Kotlin still makes itself (the open-app sweep, sync): page
 * fetches run there too, so they all share its few lanes, the page on screen first. A run outlives
 * its caller; only sign-out cancels it.
 */
object PageFetches {
    /** [result] of a run; it fails with [Dropped] when it waited off screen too long. */
    internal class Run<T>(val result: Deferred<T>)

    /**
     * [work] under [key] in [flow], when it gets a lane. [user]: the student asked (goes first
     * within its priority, never dropped); [hasCache]: there is a copy to keep if it is dropped;
     * [background]: the open-app sweep (lowest priority). Another run of [key] going already
     * counts as [Dropped].
     */
    internal fun <T> start(
        key: String,
        flow: String,
        user: Boolean = true,
        hasCache: Boolean = false,
        background: Boolean = false,
        work: suspend () -> T,
    ): Run<T> = Run(
        AppScope.async(Dispatchers.IO) {
            var outcome: Result<T>? = null
            val ran = object : QueuedWork {
                override suspend fun run(): Boolean = attempt { work() }.also { outcome = it }.isSuccess
            }
            when (AppCore.await().queueRun(flow, key, user, hasCache, background, ran)) {
                QueueEnd.RAN -> checkNotNull(outcome) { "ran without an outcome" }.getOrThrow()
                QueueEnd.JOINED, QueueEnd.DROPPED -> throw Dropped()
                QueueEnd.CANCELLED -> throw CancellationException("cancelled by sign-out")
            }
        },
    )

    /** Whether a screen shows page [id]. */
    internal fun isShown(id: String): Boolean = AppCore.ready()?.pageShown(id) ?: false

    /** The link is slow: most of the last few fetches took over [Tunable.SlowFetch], or it is UDP. */
    internal val slowLink: Boolean get() = AppCore.ready()?.slowLink() ?: false

    /** Cancels every run, waiting ones too, and forgets which succeeded (sign-out). */
    fun cancelAll() {
        AppScope.launch { attempt { coreFlow("fetch.cancel") { core, f -> core.cancelFetches(f) } } }
    }
}
