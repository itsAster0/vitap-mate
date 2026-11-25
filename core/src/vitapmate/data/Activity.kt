package vitapmate.data

import androidx.compose.runtime.Immutable
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

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

    /** What runs now, oldest first, and what finished, newest first (at most [KEEP]). */
    @Immutable
    data class State(val running: List<Entry> = emptyList(), val recent: List<Entry> = emptyList()) {
        /** The latest finished run failed because the phone is offline. */
        val offline: Boolean get() = recent.firstOrNull()?.problem?.kind == ProblemKind.Offline

        fun started(entry: Entry) = copy(running = running + entry)

        /** [entry] left [running] as [finished], at the top of [recent]. */
        fun finished(entry: Entry, finished: Entry) = State(running - entry, (listOf(finished) + recent).take(KEEP))
    }

    const val KEEP = 30

    private val now = MutableStateFlow(State())

    /** Runs start and finish on any thread: one immutable [State], swapped whole. */
    val state: StateFlow<State> = now.asStateFlow()

    /** Runs [block] as [label] (e.g. "Refreshing attendance") in [flow]; a failure is recorded and rethrown. */
    suspend fun <T> track(label: String, flow: String, block: suspend () -> T): T {
        val entry = Entry(flow, label, System.currentTimeMillis())
        now.update { it.started(entry) }
        val start = System.nanoTime()
        fun done(problem: Problem? = null, cancelled: Boolean = false) {
            val finished = entry.copy(ms = (System.nanoTime() - start) / 1_000_000, problem = problem, cancelled = cancelled)
            now.update { it.finished(entry, finished) }
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
    fun snapshot(): Map<String, Any?> = state.value.let { s ->
        mapOf(
            "running" to s.running.map { it.label },
            "recent" to s.recent.take(5).map { mapOf("label" to it.label, "ms" to it.ms, "problem" to it.problem?.kind?.name, "flow" to it.flow) },
        )
    }
}
