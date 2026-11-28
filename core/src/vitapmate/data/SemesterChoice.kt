package vitapmate.data

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.PageActivity
import vitapmate.core.SemesterState
import vitapmate.core.Semesters
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * VTOP's semesters and the one pages show, as Rust last told it
 * ([vitapmate.core.AppEvents.onSemesters]; Rust keeps the pick and a cache per semester). Pages
 * are keyed on [selected], so picking another semester reloads every open page. Nothing is
 * guessed: while [selected] is null the app shows the picker, as the Flutter app's onboarding.
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

    /** Rust's state; told in order, from Rust's threads (snapshot state takes writes from any). */
    internal fun mirror(now: SemesterState) {
        state = now
    }

    /** The cached list and pick (a page fetch caches the list too), without the network. */
    suspend fun readCache(flow: String) {
        withContext(Dispatchers.IO) { AppCore.await().semesterReadCache(flow) }
    }

    /** Fetches the list from VTOP; a failure is kept in [state], never thrown. */
    suspend fun refresh(trigger: String) = flow("semesters.refresh") { f ->
        withContext(Dispatchers.IO) { AppCore.await().semesterRefresh(f, trigger) }
    }

    /** Shows [id]'s pages from now on; outlives the picker (the sheet closes on tap). */
    fun pick(id: String) {
        AppScope.launch {
            flow("semesters.select") { f ->
                if (withContext(Dispatchers.IO) { AppCore.await().semesterPick(f, id) }) sendToServer()
            }
        }
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
