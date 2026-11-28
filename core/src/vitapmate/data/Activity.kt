package vitapmate.data

import androidx.compose.runtime.Immutable
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import vitapmate.core.ActivityEntry
import vitapmate.core.ActivityRun
import vitapmate.core.TrackEnd

/**
 * Work that talks to VTOP, for the app bar ("Refreshing attendance…") and its recent-activity
 * sheet, like the Flutter app's queue. Rust keeps the list (page fetches and the runs [track]
 * hands it); this mirrors it ([mirror]). Each entry is one flow, so "Details" opens it in the
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

    /** What runs now, oldest first, and what finished, newest first. */
    @Immutable
    data class State(val running: List<Entry> = emptyList(), val recent: List<Entry> = emptyList()) {
        /** The latest finished run failed because the phone is offline. */
        val offline: Boolean get() = recent.firstOrNull()?.problem?.kind == ProblemKind.OFFLINE
    }

    private val now = MutableStateFlow(State())

    val state: StateFlow<State> = now.asStateFlow()

    /** Rust's list: running entries first, then finished ones. */
    internal fun mirror(entries: List<ActivityEntry>) {
        val (running, recent) = entries.partition { it.run is ActivityRun.Running }
        now.value = State(running.map(::entry), recent.map(::entry))
    }

    private fun entry(e: ActivityEntry) = when (val run = e.run) {
        is ActivityRun.Running -> Entry(e.flow, e.label, e.startedAt)
        is ActivityRun.Done -> Entry(e.flow, e.label, e.startedAt, run.ms)
        is ActivityRun.Failed -> Entry(e.flow, e.label, e.startedAt, run.ms, run.problem)
        is ActivityRun.Cancelled -> Entry(e.flow, e.label, e.startedAt, run.ms, cancelled = true)
    }

    /** Runs [block] as [label] (e.g. "Refreshing calendar") in [flow]; a failure is recorded and rethrown. */
    suspend fun <T> track(label: String, flow: String, block: suspend () -> T): T {
        val core = AppCore.await()
        val id = core.trackStart(flow, label)
        return try {
            block().also { core.trackEnd(flow, id, TrackEnd.Done) }
        } catch (e: CancellationException) {
            core.trackEnd(flow, id, TrackEnd.Cancelled)
            throw e
        } catch (e: Throwable) {
            core.trackEnd(flow, id, TrackEnd.Failed(problemOf(e, flow)))
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
