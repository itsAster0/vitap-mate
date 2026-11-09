package vitapmate.data

import androidx.compose.runtime.mutableStateMapOf
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * Class histories fetched again in the background, one sync and one course at a time: the
 * attendance screen's that no longer match their summary (Flutter's `pullMismatchedHistory`) and
 * the timetable's held classes not posted yet. A failure only logs.
 */
object Histories {
    private const val RETRY_MS = 15 * 60_000L
    private val lock = Mutex()
    private val tries = mutableMapOf<String, Long>()

    /** Bumped per course key when its history lands, so an open sheet reads it again. */
    val arrived = mutableStateMapOf<String, Int>()

    /**
     * Flow `<src>.histories` (log src `ui.<src>`): fetches the course keys [due] lists; how many
     * arrived. With [semester], each course is tried at most once per [RETRY_MS] in it.
     */
    suspend fun sync(src: String, semester: String? = null, due: (Core, String) -> List<String>): Int = lock.withLock {
        flow("$src.histories") { id ->
            withContext(Dispatchers.IO) {
                val core = AppCore.await()
                val now = System.currentTimeMillis()
                val keys = due(core, id).filter { key -> semester == null || tries["$semester/$key"]?.let { now - it >= RETRY_MS } ?: true }
                if (keys.isEmpty()) return@withContext 0
                Log.info("ui.$src", "histories to fetch", id, mapOf("courses" to "${keys.size}"))
                Activity.track("Refreshing class history", id) {
                    keys.count { key ->
                        if (semester != null) tries["$semester/$key"] = System.currentTimeMillis()
                        try {
                            core.fetchAttendanceDetail(id, key)
                            withContext(Dispatchers.Main) { arrived[key] = (arrived[key] ?: 0) + 1 }
                            true
                        } catch (e: CancellationException) {
                            tries.remove("$semester/$key") // the screen moved on; the next one tries again
                            throw e
                        } catch (e: Exception) {
                            Log.warn("ui.$src", "history not fetched", id, mapOf("course" to key, "error" to (e::class.simpleName ?: "Exception")))
                            false
                        }
                    }
                }.also { Log.info("ui.$src", "histories fetched", id, mapOf("fetched" to "$it", "due" to "${keys.size}")) }
            }
        }
    }
}
