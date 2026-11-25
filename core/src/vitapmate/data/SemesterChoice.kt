package vitapmate.data

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Semesters
import vitapmate.observability.Log
import vitapmate.observability.flow

/** The semester list as the picker and Settings show it ([SemesterChoice.state]). */
sealed interface SemesterState {
    /** None cached (or not read yet), and none being fetched. */
    data object None : SemesterState

    /** None cached; fetching it. */
    data object Loading : SemesterState

    /** None cached, and the fetch failed. */
    data class Failed(val problem: Problem) : SemesterState

    /** The list, and what its refresh is doing. */
    data class Ready(val semesters: Semesters, val activity: PageActivity = PageActivity.Idle) : SemesterState
}

internal sealed interface SemesterEvent {
    /** The cached list (null: none, e.g. after sign-out). */
    data class CacheRead(val semesters: Semesters?) : SemesterEvent
    data object RefreshStarted : SemesterEvent
    data class RefreshSucceeded(val semesters: Semesters) : SemesterEvent
    data class RefreshFailed(val problem: Problem) : SemesterEvent

    /** The refresh stopped without an answer (its screen went). */
    data object RefreshCancelled : SemesterEvent
}

internal fun SemesterState.next(event: SemesterEvent): SemesterState = when (event) {
    is SemesterEvent.CacheRead -> when {
        event.semesters != null -> SemesterState.Ready(
            event.semesters,
            when (this) {
                is SemesterState.Ready -> activity
                SemesterState.Loading -> PageActivity.Refreshing()
                is SemesterState.Failed -> PageActivity.Stale(problem)
                SemesterState.None -> PageActivity.Idle
            },
        )
        // The list went (sign-out); a refresh may still bring one.
        this is SemesterState.Ready -> if (activity is PageActivity.Refreshing) SemesterState.Loading else SemesterState.None
        else -> this
    }
    SemesterEvent.RefreshStarted -> if (this is SemesterState.Ready) copy(activity = PageActivity.Refreshing()) else SemesterState.Loading
    is SemesterEvent.RefreshSucceeded -> SemesterState.Ready(event.semesters)
    is SemesterEvent.RefreshFailed -> if (this is SemesterState.Ready) copy(activity = PageActivity.Stale(event.problem)) else SemesterState.Failed(event.problem)
    SemesterEvent.RefreshCancelled -> when (this) {
        is SemesterState.Ready -> copy(activity = activity.settled())
        SemesterState.Loading -> SemesterState.None
        SemesterState.None, is SemesterState.Failed -> this
    }
}

/**
 * VTOP's semesters and the one pages show (Rust keeps the pick and a cache per semester). Pages
 * are keyed on [selected], so picking another semester reloads every open page. Nothing is
 * guessed: while [selected] is null the app shows the picker, as the Flutter app's onboarding.
 * One [state], changed by [SemesterEvent]s from any thread.
 */
object SemesterChoice {
    var state by mutableStateOf<SemesterState>(SemesterState.None)
        private set

    val semesters: Semesters? get() = (state as? SemesterState.Ready)?.semesters

    val selected: String? get() = semesters?.selected

    /** A fetch of the list runs. */
    val loading: Boolean
        get() = state.let { it is SemesterState.Loading || (it as? SemesterState.Ready)?.activity is PageActivity.Refreshing }

    /** Why the last fetch of the list failed, until one succeeds. */
    val problem: Problem?
        get() = when (val s = state) {
            is SemesterState.Failed -> s.problem
            is SemesterState.Ready -> (s.activity as? PageActivity.Stale)?.problem
            SemesterState.None, SemesterState.Loading -> null
        }

    private fun on(event: SemesterEvent) = synchronized(this) { state = state.next(event) }

    /** The cached list and pick (a page fetch caches the list too), without the network. */
    suspend fun readCache(flow: String) {
        on(SemesterEvent.CacheRead(withContext(Dispatchers.IO) { AppCore.await().cachedSemesters(flow) }))
    }

    /** Fetches the list from VTOP; a failure is kept in [state], never thrown. */
    suspend fun refresh(trigger: String) = flow("semesters.refresh") { f ->
        on(SemesterEvent.RefreshStarted)
        try {
            attempt {
                Activity.track("Loading semesters", f) { withContext(Dispatchers.IO) { AppCore.await().fetchSemesters(f) } }
            }.onSuccess { fresh ->
                on(SemesterEvent.RefreshSucceeded(fresh))
                Log.info("ui.semesters", "fresh", f, mapOf("trigger" to trigger, "count" to "${fresh.items.size}"))
            }.onFailure { e ->
                val failed = problemOf(e, f)
                on(SemesterEvent.RefreshFailed(failed))
                askedOtp(e)
                Log.warn("ui.semesters", "refresh failed", f, mapOf("trigger" to trigger, "kind" to failed.kind.name))
            }
        } catch (e: CancellationException) {
            on(SemesterEvent.RefreshCancelled)
            throw e
        }
    }

    /** Shows [id]'s pages from now on; outlives the picker (the sheet closes on tap). */
    fun pick(id: String) {
        AppScope.launch { select(id) }
    }

    private suspend fun select(id: String) = flow("semesters.select") { f ->
        if (id == selected) return@flow
        withContext(Dispatchers.IO) { AppCore.await().selectSemester(f, id) }
        readCache(f)
        Log.info("ui.semesters", "selected", f, mapOf("semester" to id))
        sendToServer()
    }

    /** Connected apps read the semester picked here; best effort (skipped before a key is made). */
    private fun sendToServer() {
        AppScope.launch {
            flow("semesters.server") { f ->
                attempt { withContext(Dispatchers.IO) { AppCore.await().sendServerSemester(f) } }
                    .onFailure { Log.info("ui.semesters", "not sent to the server: ${it::class.simpleName}", f) }
            }
        }
    }
}
