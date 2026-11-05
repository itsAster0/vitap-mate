package vitapmate.data

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.CoreException
import vitapmate.core.Semesters
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * VTOP's semesters and the one pages show (Rust keeps the pick and a cache per semester). Pages
 * are keyed on [selected], so picking another semester reloads every open page. Nothing is
 * guessed: while [selected] is null the app shows the picker, as the Flutter app's onboarding.
 */
object SemesterChoice {
    var semesters by mutableStateOf<Semesters?>(null)
        private set
    var loading by mutableStateOf(false)
        private set
    var problem by mutableStateOf<Problem?>(null)
        private set

    val selected: String? get() = semesters?.selected

    /** The cached list and pick (a page fetch caches the list too), without the network. */
    suspend fun readCache(flow: String) {
        semesters = withContext(Dispatchers.IO) { AppCore.await().cachedSemesters(flow) }
    }

    /** Fetches the list from VTOP; a failure is kept in [problem], never thrown. */
    suspend fun refresh(trigger: String) = flow("semesters.refresh") { f ->
        loading = true
        try {
            semesters = Activity.track("Loading semesters", f) { withContext(Dispatchers.IO) { AppCore.await().fetchSemesters(f) } }
            problem = null
            Log.info("ui.semesters", "fresh", f, mapOf("trigger" to trigger, "count" to "${semesters?.items?.size}"))
        } catch (e: Exception) {
            if (e is kotlinx.coroutines.CancellationException) throw e
            problem = problemOf(e, f)
            if (e is CoreException.OtpRequired) OtpPrompt.ask(e.reason)
            Log.warn("ui.semesters", "refresh failed", f, mapOf("trigger" to trigger, "kind" to problem!!.kind.name))
        } finally {
            loading = false
        }
    }

    // Outlives the picker: the sheet closes as soon as a semester is tapped.
    private val scope = MainScope()

    /** Shows [id]'s pages from now on; runs on, whatever screen asked. */
    fun pick(id: String) {
        scope.launch { select(id) }
    }

    private suspend fun select(id: String) = flow("semesters.select") { f ->
        if (id == selected) return@flow
        withContext(Dispatchers.IO) { AppCore.await().selectSemester(f, id) }
        readCache(f)
        Log.info("ui.semesters", "selected", f, mapOf("semester" to id))
    }
}
