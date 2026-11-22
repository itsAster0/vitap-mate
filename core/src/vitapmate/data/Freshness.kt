package vitapmate.data

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.batchedPages
import vitapmate.observability.Log
import vitapmate.observability.flow
import kotlin.time.Duration.Companion.milliseconds

/** Whether the app is on screen, and how often it came back (open pages then refresh if old). */
object Foreground {
    var resumes by mutableIntStateOf(0)
        private set
    /** Activities started and not stopped: a new MainActivity can start before the old one stops. */
    @Volatile private var started = 0
    val shown get() = started > 0
    private var left = false

    /** From the activity's `onStart`. */
    fun started() {
        started++
        if (left) resumes++
    }

    /** From the activity's `onStop`. */
    fun stopped() {
        started = (started - 1).coerceAtLeast(0)
        if (shown) return
        left = true
        CookieConsent.left()
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
        flow("$src.calendar", unattended = src == "sync") { id ->
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
                val today = epochDayOf(now)
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
 * not on screen that are older than 12 h are fetched: attendance, timetable, marks, exams and the
 * semester list as one batch (one round trip on the Server and UDP routes, [Core.fetchPages]), then
 * the calendar when a day old, then histories behind their summary one at a time. It
 * doesn't start on a slow link ([PageFetches.slowLink]), waits while anything else talks to VTOP,
 * takes [PageFetches]' lanes at the lowest priority (a page the student opens goes first) and
 * stops at the first failure, slow or dropped page, so it never competes with the student; an OTP
 * it runs into is not prompted (the student's next fetch asks).
 * Times are [Tunable]s (Settings → Developer): [Tunable.SweepQuick], [Tunable.SweepCooldown],
 * [Tunable.SweepMaxAge], [Tunable.SlowPage] (Flutter's background page timeout).
 * Flow `sync.stale`, log src `sync.stale`.
 */
object StaleSweep {
    /** After the student's requests, wait this long in case more follow. */
    private const val QUIET_MS = 2_000L

    private var job: Job? = null
    private var lastRun = 0L

    /** A page fetch of [ms] succeeded. */
    internal fun afterFetch(ms: Long) {
        val now = System.currentTimeMillis()
        if (ms > Settings.tunable(Tunable.SweepQuick) * 1000L || PageFetches.slowLink || !Settings.autoRefresh || !Foreground.shown) return
        if (job?.isActive == true || now - lastRun < Settings.tunable(Tunable.SweepCooldown) * 60_000L) return
        lastRun = now
        job = AppScope.launch { sweep() }
    }

    private class Stop(val reason: String) : Exception(reason)

    private suspend fun sweep() = flow("sync.stale", unattended = true) { f ->
        val core = AppCore.await()
        var fetched = 0
        /** Page [id]'s saved time (0 for none), or null when it is on screen or new enough. */
        suspend fun due(id: String, updatedAt: (Core) -> Long?): Long? {
            // Pages on screen refresh themselves.
            if (PageFetches.isShown(id)) return null
            val at = withContext(Dispatchers.IO) { updatedAt(core) } ?: return 0
            return at.takeIf { System.currentTimeMillis() / 1000 - it >= Settings.tunable(Tunable.SweepMaxAge) * 3600L }
        }
        /** Fetches the pages saved at [ats] (one, or a batch as [id]), each given [Tunable.SlowPage]. */
        suspend fun run(id: String, label: String, ats: List<Long>, fetch: suspend (Core) -> Unit) {
            snapshotFlow { Activity.running.isEmpty() }.first { it }
            delay(QUIET_MS.milliseconds)
            if (!Settings.autoRefresh || !Foreground.shown) throw Stop("app left or auto refresh off")
            val ms = try {
                PageFetches.start("$id/stale/", f, user = false, hasCache = ats.none { it == 0L }, background = true) {
                    val start = System.nanoTime()
                    Activity.track(label, f) { withContext(Dispatchers.IO) { fetch(core) } }
                    (System.nanoTime() - start) / 1_000_000
                }.result.await()
            } catch (e: CancellationException) {
                throw e
            } catch (_: Dropped) {
                throw Stop("$id waited too long")
            } catch (e: Exception) {
                BackgroundProblems.report(e, f)
                throw Stop("$id failed: ${e::class.simpleName}")
            }
            fetched += ats.size
            val ages = ats.joinToString(",") { if (it == 0L) "null" else "${System.currentTimeMillis() / 1000 - it}" }
            Log.info("sync.stale", "fetched $id", f, mapOf("page" to id, "ms" to "$ms", "age_s" to ages))
            if (ms > Settings.tunable(Tunable.SlowPage) * 1000L * ats.size) throw Stop("network is slow (${ms} ms for ${ats.size} pages)")
        }
        suspend fun step(id: String, label: String, updatedAt: (Core) -> Long?, fetch: suspend (Core) -> Unit) {
            due(id, updatedAt)?.let { run(id, label, listOf(it), fetch) }
        }
        try {
            // One round trip on the Server and UDP routes: (page id, Rust's page name, saved time).
            // Only pages Rust batches (route/batch.rs); the calendar refreshes on its own below.
            val batched = batchedPages().toSet()
            val batch = listOf<Triple<String, String, (Core) -> Long?>>(
                Triple("attendance", "attendance") { it.cachedAttendance(f)?.updatedAt },
                Triple("timetable", "timetable") { it.cachedTimetable(f)?.updatedAt },
                Triple("marks", "marks") { it.cachedMarks(f)?.updatedAt },
                Triple("exams", "exam_schedule") { it.cachedExams(f)?.updatedAt },
                Triple("semesters", "semesters") { it.cachedSemesters(f)?.updatedAt },
            ).filter { (id, name) ->
                (name in batched).also { if (!it) Log.warn("sync.batch", "$name is not batched in Rust", f, mapOf("page" to id)) }
            }.mapNotNull { (id, name, updatedAt) -> due(id, updatedAt)?.let { Triple(id, name, it) } }
            if (batch.isNotEmpty()) {
                val ids = batch.joinToString("+") { it.first }
                run(ids, "Refreshing ${batch.joinToString(", ") { it.first }}", batch.map { it.third }) { c ->
                    c.fetchPages(f, batch.map { it.second }).firstNotNullOfOrNull { it.error }?.let { throw it }
                }
            }
            SemesterChoice.selected?.let { CalendarRefresh.ifOld(it, "sync", anyway = true) }
            SemesterChoice.readCache(f)
            if (!PageFetches.isShown("attendance")) {
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
