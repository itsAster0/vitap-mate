package vitapmate.features.attendance

import androidx.compose.runtime.mutableStateMapOf
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.data.Activity
import vitapmate.data.AppCore
import vitapmate.observability.Log
import vitapmate.observability.flow

/** Class histories fetched again after a summary refresh (Flutter's `pullMismatchedHistory`). */
internal object Histories {
    /** Bumped per course key when its history lands, so an open sheet reads it again. */
    val arrived = mutableStateMapOf<String, Int>()

    /**
     * Fetches again, one course at a time (flow `attendance.histories`), the history of each
     * course whose saved history is missing or no longer matches its summary. A failure only
     * logs. Rust keeps one fetch per course at a time, so this never
     * doubles the timetable's sync.
     */
    suspend fun syncMismatched() = flow("attendance.histories") { id ->
        withContext(Dispatchers.IO) {
            val core = AppCore.await()
            val due = core.mismatchedAttendanceDetails(id)
            if (due.isEmpty()) return@withContext
            Log.info("ui.attendance", "histories behind", id, mapOf("courses" to "${due.size}"))
            val fetched = Activity.track("Refreshing class history", id) {
                due.count { key ->
                    try {
                        core.fetchAttendanceDetail(id, key)
                        withContext(Dispatchers.Main) { arrived[key] = (arrived[key] ?: 0) + 1 }
                        true
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Exception) {
                        Log.warn("ui.attendance", "history not fetched", id, mapOf("course" to key, "error" to (e::class.simpleName ?: "Exception")))
                        false
                    }
                }
            }
            Log.info("ui.attendance", "histories fetched", id, mapOf("fetched" to "$fetched", "due" to "${due.size}"))
        }
    }
}
