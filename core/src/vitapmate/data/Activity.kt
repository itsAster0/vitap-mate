package vitapmate.data

import androidx.compose.runtime.Immutable
import androidx.compose.runtime.mutableStateListOf
import kotlin.coroutines.cancellation.CancellationException

/**
 * Work that talks to VTOP, for the app bar ("Refreshing attendance…") and its recent-activity
 * sheet, like the Flutter app's queue. Each entry is one flow, so "Details" opens it in the
 * Inspector.
 */
object Activity {
    @Immutable
    data class Entry(
        val flow: String,
        val label: String,
        val startedAt: Long,
        val ms: Long? = null,
        val problem: Problem? = null,
        val cancelled: Boolean = false,
    )

    private const val KEEP = 30

    /** Running now, oldest first. */
    val running = mutableStateListOf<Entry>()

    /** Finished, newest first (at most 30). */
    val recent = mutableStateListOf<Entry>()

    /** The latest finished run failed because the phone is offline. */
    val offline: Boolean get() = recent.firstOrNull()?.problem?.kind == ProblemKind.Offline

    /** Runs [block] as [label] (e.g. "Refreshing attendance") in [flow]; a failure is recorded and rethrown. */
    suspend fun <T> track(label: String, flow: String, block: suspend () -> T): T {
        val entry = Entry(flow, label, System.currentTimeMillis())
        running += entry
        val start = System.nanoTime()
        fun done(problem: Problem? = null, cancelled: Boolean = false) {
            running -= entry
            recent.add(0, entry.copy(ms = (System.nanoTime() - start) / 1_000_000, problem = problem, cancelled = cancelled))
            while (recent.size > KEEP) recent.removeAt(recent.lastIndex)
        }
        return try {
            block().also { done() }
        } catch (e: CancellationException) {
            done(cancelled = true)
            throw e
        } catch (e: Throwable) {
            done(problemOf(e, flow))
            throw e
        }
    }

    /** For the state dump. */
    fun snapshot(): Map<String, Any?> = mapOf(
        "running" to running.map { it.label },
        "recent" to recent.take(5).map { mapOf("label" to it.label, "ms" to it.ms, "problem" to it.problem?.kind?.name, "flow" to it.flow) },
    )
}
