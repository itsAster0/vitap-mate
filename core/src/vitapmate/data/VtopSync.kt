package vitapmate.data

import java.util.concurrent.atomic.AtomicInteger
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.core.OutingStanding
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * Fetches VTOP's pages for background sync and More → Fetch all: semesters and attendance first,
 * then the rest through [PageFetches]' lanes (several at once on a fast link; Fetch all as the
 * student's, background sync at the lowest priority). Each page gets [pageTimeoutMs] once it has a
 * lane; a page that fails is logged and skipped (the others still run). When VTOP asks for the
 * OTP, [otp] must get it accepted (background: auto-fill alone; Fetch all: the prompt), asked once
 * however many pages need it, and returns false to stop the whole run. [everything] adds the pages background sync
 * leaves out (profile, course page, outings). Rust records what changed on each fetch (the
 * change inbox). Flow [flowName] ([unattended] for background sync: it uses the chosen route when
 * its "Don't ask again" is ticked, else Direct), log src `sync`.
 */
object VtopSync {
    /** How a run ended. */
    enum class Outcome { Done, StoppedAtOtp, Failed }

    data class Result(val outcome: Outcome, val fetched: Int, val failed: Int)

    suspend fun run(
        flowName: String,
        label: String,
        everything: Boolean,
        pageTimeoutMs: Long,
        unattended: Boolean = false,
        otp: suspend (CoreException.OtpRequired, String) -> Boolean,
    ): Result = flow(flowName, unattended) { f ->
        val core = try {
            AppCore.await()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("sync", "no core: ${e::class.simpleName}", f)
            return@flow Result(Outcome.Failed, 0, 0)
        }
        val fetched = AtomicInteger()
        val failed = AtomicInteger()
        class Stop : Exception()
        class TimedOut : Exception()
        // Pages that run into the OTP together share one prompt: the first asks, the rest wait for
        // it and try again; [otpRounds] counts the codes accepted, [stopped] that one wasn't.
        val otpPrompt = Mutex()
        var otpRounds = 0
        var stopped = false

        // Through PageFetches' lanes: several at once on a fast link, one on a slow one or UDP.
        suspend fun page(id: String, fetch: suspend (Core) -> Unit) {
            val name = id.substringBefore('.')
            val key = "${if (name == "history") "attendance_detail" else name}/sync/${id.substringAfter('.', "")}"
            // One OTP round at most per page: after it is accepted, the page is fetched again.
            repeat(2) { attempt ->
                val round = otpRounds
                try {
                    PageFetches.start(key, f, user = everything, background = !everything) {
                        Activity.track("$label: $name", f) {
                            withTimeoutOrNull(pageTimeoutMs) { withContext(Dispatchers.IO) { fetch(core) } } ?: throw TimedOut()
                        }
                    }.result.await()
                    fetched.incrementAndGet()
                    return
                } catch (e: CancellationException) {
                    throw e
                } catch (_: TimedOut) {
                    failed.incrementAndGet()
                    Log.warn("sync", "page timed out", f, mapOf("page" to id))
                    return
                } catch (e: CoreException.OtpRequired) {
                    if (attempt > 0) throw Stop()
                    otpPrompt.withLock {
                        if (stopped) throw Stop()
                        if (otpRounds == round) {
                            if (!otp(e, f)) {
                                stopped = true
                                throw Stop()
                            }
                            otpRounds++
                        }
                    }
                } catch (e: Throwable) {
                    failed.incrementAndGet()
                    Log.warn("sync", "page failed: ${e::class.simpleName}", f, mapOf("page" to id))
                    return
                }
            }
        }

        try {
            // Semesters and attendance first: the pick and the class histories to fetch come from them.
            page("semesters") { it.fetchSemesters(f) }
            SemesterChoice.readCache(f)
            page("attendance") { it.fetchAttendance(f) }
            val pages = buildList<Pair<String, suspend (Core) -> Unit>> {
                for (key in withContext(Dispatchers.IO) { core.mismatchedAttendanceDetails(f) }) add("history.$key" to { it.fetchAttendanceDetail(f, key) })
                add("timetable" to { it.fetchTimetable(f) })
                add("marks" to { it.fetchMarks(f) })
                add("exams" to { it.fetchExams(f) })
                add("calendar" to { it.fetchCalendar(f) })
                SemesterChoice.selected?.let { semester -> add("grades" to { it.fetchGrades(f, semester) }) }
                add("grade_history" to { it.fetchGradeHistory(f) })
                // Recent biometric days not final yet, so campus times build up over the semester.
                val today = Math.floorDiv(System.currentTimeMillis() + 19_800_000L, DAY_MS).toInt() // India
                val complete = withContext(Dispatchers.IO) { core.cachedBiometricRange(f, today - 13, today) }.filter { it.complete }.mapTo(HashSet()) { it.day }
                for (day in biometricDays(today, complete, Settings.tunable(Tunable.SyncBiometricDays))) add("biometric.$day" to { it.fetchBiometricRange(f, day, day) })
                if (everything) {
                    add("profile" to { it.fetchProfile(f) })
                    add("course_page" to { it.fetchCoursePage(f) })
                } else {
                    // One course's lecture plan a run, in turn, so new files reach the inbox.
                    withContext(Dispatchers.IO) { nextCourse(core, f) }?.let { id -> add("course_files" to { it.fetchCourseView(f, id, null) }) }
                }
                // Outings are checked while a trip ahead is pending, so a decision shows in the inbox (and Live updates).
                if (everything || withContext(Dispatchers.IO) { pendingOuting(core, f) }) {
                    add("general_outing" to { it.fetchGeneralOuting(f) })
                    add("weekend_outing" to { it.fetchWeekendOuting(f) })
                }
            }
            // The rest in any order; the lanes keep them in this one.
            coroutineScope { pages.map { (id, fetch) -> async { page(id, fetch) } }.awaitAll() }
        } catch (_: Stop) {
            Log.info("sync", "stopped at the otp", f, mapOf("fetched" to "$fetched", "failed" to "$failed"))
            return@flow Result(Outcome.StoppedAtOtp, fetched.get(), failed.get())
        }
        Log.info("sync", "done", f, mapOf("fetched" to "$fetched", "failed" to "$failed"))
        Result(Outcome.Done, fetched.get(), failed.get())
    }

    /** The newest [n] of the 14 days to [today] not in [complete], newest first; today always counts. */
    internal fun biometricDays(today: Int, complete: Set<Int>, n: Int): List<Int> =
        (today downTo today - 13).filter { it == today || it !in complete }.take(n)

    /** The course part after the one checked last ([Settings.syncCourse]), from the saved course list; null without one. */
    internal fun nextCourse(core: Core, f: String): String? {
        val ids = core.cachedCoursePage(f)?.groups.orEmpty().flatMap { g -> g.parts.map { it.id } }
        return nextAfter(ids, Settings.syncCourse)?.also { Settings.syncCourse = it }
    }

    /** The id after [last] in [ids], wrapping round; the first when [last] is gone. */
    internal fun nextAfter(ids: List<String>, last: String?): String? =
        if (ids.isEmpty()) null else ids[(ids.indexOf(last) + 1) % ids.size]

    private fun pendingOuting(core: Core, f: String) =
        (core.cachedGeneralOuting(f)?.requests.orEmpty() + core.cachedWeekendOuting(f)?.requests.orEmpty()).any { it.standing == OutingStanding.PENDING && it.highlight != null }
}
