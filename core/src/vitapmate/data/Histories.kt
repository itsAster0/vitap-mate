package vitapmate.data

import androidx.compose.runtime.mutableStateMapOf
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.observability.flow

/**
 * Class histories fetched again in the background (`Core.historiesRefresh`: one batch at a time,
 * a failure only logs): the attendance screen's that no longer match their summary and the
 * timetable's held classes not posted yet.
 */
object Histories {
    /** Bumped per course key when its history lands, so an open sheet reads it again. */
    val arrived = mutableStateMapOf<String, Int>()

    /**
     * Flow `<src>.histories` (log src `ui.<src>`): fetches the course keys [due] lists; how many
     * arrived. With [semester], each course is tried at most once per 15 minutes in it.
     */
    suspend fun sync(src: String, semester: String? = null, due: (Core, String) -> List<String>): Int = flow("$src.histories") { id ->
        val core = AppCore.await()
        val got = withContext(Dispatchers.IO) { core.historiesRefresh(id, src, semester, due(core, id)) }
        withContext(Dispatchers.Main) { got.forEach { arrived[it] = (arrived[it] ?: 0) + 1 } }
        got.size
    }
}
