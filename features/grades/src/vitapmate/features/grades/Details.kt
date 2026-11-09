package vitapmate.features.grades

import androidx.compose.runtime.Stable
import androidx.compose.runtime.mutableStateMapOf
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.GradeDetails
import vitapmate.data.Activity
import vitapmate.data.AppCore
import vitapmate.data.askedOtp
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.showToast

/**
 * Each course's mark breakdown, by `<semester>/<course id>`, as the Flutter grades page keeps
 * them: opening a card reads the saved one and fetches only when none is saved or a cut-off is
 * VTOP's "#" placeholder; the refresh button always fetches. Flow `grades.details`.
 */
@Stable
internal class Details {
    val loaded = mutableStateMapOf<String, GradeDetails>()
    val loading = mutableStateMapOf<String, Unit>()
    val failed = mutableStateMapOf<String, Unit>()

    suspend fun open(semester: String, courseId: String) {
        val key = "$semester/$courseId"
        if (key in loading || loaded[key]?.placeholder == false) return
        flow("grades.details") { id ->
            val saved = loaded[key] ?: withContext(Dispatchers.IO) { AppCore.await().cachedGradeDetails(id, semester, courseId) }
            if (saved != null) loaded[key] = saved
            Log.info("ui.grades", "details open", id, mapOf("course" to courseId, "saved" to "${saved != null}", "placeholder" to "${saved?.placeholder}"))
            if (saved == null || saved.placeholder) fetch(id, semester, courseId, "open")
        }
    }

    suspend fun refresh(semester: String, courseId: String) = flow("grades.details") { id -> fetch(id, semester, courseId, "button") }

    private suspend fun fetch(id: String, semester: String, courseId: String, trigger: String) {
        val key = "$semester/$courseId"
        if (key in loading) return
        loading[key] = Unit
        try {
            val fresh = Activity.track("Refreshing grade details", id) {
                withContext(Dispatchers.IO) { AppCore.await().fetchGradeDetails(id, semester, courseId) }
            }
            loaded[key] = fresh
            failed.remove(key)
            Log.info("ui.grades", "details fresh", id, mapOf("course" to courseId, "trigger" to trigger, "sections" to "${fresh.sections.size}"))
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            failed[key] = Unit
            askedOtp(e)
            val problem = problemOf(e, id).copy(userInitiated = trigger == "button")
            Log.warn("ui.grades", "details not fetched", id, mapOf("course" to courseId, "trigger" to trigger, "kind" to problem.kind.name))
            // Only for the button (or with "Show background errors"); never for an OTP request.
            showToast(problem)
        } finally {
            loading.remove(key)
        }
    }
}
