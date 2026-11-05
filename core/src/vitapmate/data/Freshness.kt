package vitapmate.data

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.observability.Log
import vitapmate.observability.flow
import kotlin.time.Duration.Companion.milliseconds

/** Whether the app is on screen, and how often it came back (open pages then refresh if old). */
object Foreground {
    var resumes by mutableIntStateOf(0)
        private set
    var shown = false
        private set
    private var left = false

    /** From the activity's `onStart`. */
    fun started() {
        shown = true
        if (left) resumes++
    }

    /** From the activity's `onStop`. */
    fun stopped() {
        shown = false
        left = true
    }
}

/**
 * The academic calendar fetched again when none is saved or the saved one is older than a day
 * ([Tunable.CalendarMaxAge])
 * (the Flutter app's `calendarMaxAgeOnAttendance`: catches holidays announced mid-semester).
 * Callers: the timetable when shown, attendance after each refresh, the open-app sweep.
 */
object CalendarRefresh {
    /** One fetch at a time, at most one try per semester in this long (offline, VTOP down). */
    private const val RETRY_MS = 15 * 60_000L
    private val lock = Mutex()
    private val tries = mutableMapOf<String, Long>()

    /**
     * Fetches [semester]'s calendar if old, in flow `<src>.calendar` (log src `ui.<src>`); true when
     * a fresh one arrived. With Auto Refresh off only a missing calendar is fetched, unless
     * [anyway] (after an attendance refresh, as in Flutter). Failures log ([BackgroundProblems]).
     */
    suspend fun ifOld(semester: String, src: String, anyway: Boolean = false): Boolean = lock.withLock {
        flow("$src.calendar") { id ->
            withContext(Dispatchers.IO) {
                val core = AppCore.await()
                val saved = core.cachedCalendar(id)
                val age = saved?.let { System.currentTimeMillis() / 1000 - it.updatedAt }
                val tried = tries[semester]
                if (age != null && (age < Settings.tunable(Tunable.CalendarMaxAge) * 3600L || !(anyway || Settings.autoRefresh))) return@withContext false
                if (tried != null && System.currentTimeMillis() - tried < RETRY_MS) return@withContext false
                tries[semester] = System.currentTimeMillis()
                try {
                    val fresh = Activity.track("Refreshing calendar", id) { core.fetchCalendar(id) }
                    Log.info("ui.$src", "calendar fetched", id, mapOf("months" to "${fresh.months.size}", "age_s" to "$age"))
                    true
                } catch (e: CancellationException) {
                    tries.remove(semester) // the screen moved on; the next one tries again
                    throw e
                } catch (e: Exception) {
                    Log.warn("ui.$src", "calendar not fetched", id, mapOf("error" to (e::class.simpleName ?: "Exception"), "age_s" to "$age"))
                    BackgroundProblems.report(e, id)
                    false
                }
            }
        }
    }
}

/**
 * The exam schedule fetched again on a day with one of the student's papers, so a moved venue,
 * seat or time shows (the Flutter timetable's exam-day refresh). Only with Auto Refresh on and a
 * saved schedule; at most one try per semester in [RETRY_MS]. Caller: the timetable when shown.
 */
object ExamDayRefresh {
    private const val RETRY_MS = 15 * 60_000L
    private val lock = Mutex()
    private val tries = mutableMapOf<String, Long>()

    /** Flow `<src>.exams` (log src `ui.<src>`); true when a fresh schedule arrived. Failures log ([BackgroundProblems]). */
    suspend fun ifExamToday(semester: String, src: String): Boolean = lock.withLock {
        if (!Settings.autoRefresh) return@withLock false
        flow("$src.exams") { id ->
            withContext(Dispatchers.IO) {
                val core = AppCore.await()
                val now = System.currentTimeMillis()
                val today = Math.floorDiv(now + java.util.TimeZone.getDefault().getOffset(now), 86_400_000L).toInt()
                val saved = core.cachedExams(id) ?: return@withContext false
                if (saved.groups.none { g -> g.papers.any { it.day == today } }) return@withContext false
                if (tries[semester]?.let { now - it < RETRY_MS } == true) return@withContext false
                tries[semester] = now
                try {
                    val fresh = Activity.track("Refreshing exams", id) { core.fetchExams(id) }
                    Log.info("ui.$src", "exams fetched (exam day)", id, mapOf("papers" to "${fresh.groups.sumOf { it.papers.size }}"))
                    true
                } catch (e: CancellationException) {
                    tries.remove(semester)
                    throw e
                } catch (e: Exception) {
                    Log.warn("ui.$src", "exams not fetched", id, mapOf("error" to (e::class.simpleName ?: "Exception")))
                    BackgroundProblems.report(e, id)
                    false
                }
            }
        }
    }
}

/**
 * The Flutter app's open-app stale refresher: after a quick page fetch (the session is live and
 * the network good), at most every 15 min, while the app is shown and Auto Refresh is on, pages
 * not on screen that are older than 12 h are fetched one at a time (attendance, then the calendar
 * when a day old, timetable, marks, exams, the semester list, histories behind their summary). It waits
 * while anything else talks to VTOP and stops at the first failure or slow page, so it never
 * competes with the student; an OTP it runs into is not prompted (the student's next fetch asks).
 * Times are [Tunable]s (Settings → Developer): [Tunable.SweepQuick], [Tunable.SweepCooldown],
 * [Tunable.SweepMaxAge], [Tunable.SlowPage] (Flutter's background page timeout).
 * Flow `sync.stale`, log src `sync.stale`.
 */
object StaleSweep {
    /** After the student's requests, wait this long in case more follow. */
    private const val QUIET_MS = 2_000L

    private val scope = MainScope()
    private var job: Job? = null
    private var lastRun = 0L

    /** Page ids on screen; they refresh themselves. */
    private val onScreen = mutableMapOf<String, Int>()

    internal fun shown(id: String, on: Boolean) {
        onScreen[id] = (onScreen[id] ?: 0) + if (on) 1 else -1
        if (onScreen[id]!! <= 0) onScreen.remove(id)
    }

    /** A page fetch of [ms] succeeded. */
    internal fun afterFetch(ms: Long) {
        val now = System.currentTimeMillis()
        if (ms > Settings.tunable(Tunable.SweepQuick) * 1000L || !Settings.autoRefresh || !Foreground.shown) return
        if (job?.isActive == true || now - lastRun < Settings.tunable(Tunable.SweepCooldown) * 60_000L) return
        lastRun = now
        job = scope.launch { sweep() }
    }

    private class Stop(val reason: String) : Exception(reason)

    private suspend fun sweep() = flow("sync.stale") { f ->
        val core = AppCore.await()
        var fetched = 0
        suspend fun step(id: String, label: String, updatedAt: (Core) -> Long?, fetch: suspend (Core) -> Unit) {
            if (id in onScreen) return
            val at = withContext(Dispatchers.IO) { updatedAt(core) }
            if (at != null && System.currentTimeMillis() / 1000 - at < Settings.tunable(Tunable.SweepMaxAge) * 3600L) return
            snapshotFlow { Activity.running.isEmpty() }.first { it }
            delay(QUIET_MS.milliseconds)
            if (!Settings.autoRefresh || !Foreground.shown) throw Stop("app left or auto refresh off")
            val start = System.nanoTime()
            try {
                Activity.track(label, f) { withContext(Dispatchers.IO) { fetch(core) } }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                BackgroundProblems.report(e, f)
                throw Stop("$id failed: ${e::class.simpleName}")
            }
            fetched++
            val ms = (System.nanoTime() - start) / 1_000_000
            Log.info("sync.stale", "fetched $id", f, mapOf("page" to id, "ms" to "$ms", "age_s" to "${at?.let { System.currentTimeMillis() / 1000 - it }}"))
            if (ms > Settings.tunable(Tunable.SlowPage) * 1000L) throw Stop("network is slow (${ms} ms page)")
        }
        try {
            step("attendance", "Refreshing attendance", { it.cachedAttendance(f)?.updatedAt }) { it.fetchAttendance(f) }
            SemesterChoice.selected?.let { CalendarRefresh.ifOld(it, "sync", anyway = true) }
            step("timetable", "Refreshing timetable", { it.cachedTimetable(f)?.updatedAt }) { it.fetchTimetable(f) }
            step("marks", "Refreshing marks", { it.cachedMarks(f)?.updatedAt }) { it.fetchMarks(f) }
            step("exams", "Refreshing exams", { it.cachedExams(f)?.updatedAt }) { it.fetchExams(f) }
            step("semesters", "Loading semesters", { it.cachedSemesters(f)?.updatedAt }) { it.fetchSemesters(f) }
            SemesterChoice.readCache(f)
            if ("attendance" !in onScreen) {
                for (key in withContext(Dispatchers.IO) { core.mismatchedAttendanceDetails(f) }) {
                    step("history.$key", "Refreshing class history", { null }) { it.fetchAttendanceDetail(f, key) }
                }
            }
            Log.info("sync.stale", "done", f, mapOf("fetched" to "$fetched"))
        } catch (stop: Stop) {
            Log.info("sync.stale", "stopped: ${stop.reason}", f, mapOf("fetched" to "$fetched"))
        }
    }
}
