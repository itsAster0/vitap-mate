package vitapmate.data

import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.observability.Log
import vitapmate.observability.flow

/**
 * Fetches VTOP's pages one after another, for background sync and More → Fetch all. Each page
 * gets [pageTimeoutMs]; a page that fails is logged and skipped (the others still run). When VTOP
 * asks for the OTP, [otp] must get it accepted (background: auto-fill alone; Fetch all: the
 * prompt) and returns false to stop the whole run. [everything] adds the pages background sync
 * leaves out (profile, course page, outings). Rust records what changed on each fetch (the
 * change inbox). Flow [flowName], log src `sync`.
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
        otp: suspend (CoreException.OtpRequired, String) -> Boolean,
    ): Result = flow(flowName) { f ->
        val core = try {
            AppCore.await()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("sync", "no core: ${e::class.simpleName}", f)
            return@flow Result(Outcome.Failed, 0, 0)
        }
        var fetched = 0
        var failed = 0
        class Stop : Exception()

        suspend fun page(id: String, fetch: suspend (Core) -> Unit) {
            // One OTP round at most per page: after it is accepted, the page is fetched again.
            repeat(2) { attempt ->
                try {
                    Activity.track("$label: ${id.substringBefore('.')}", f) {
                        withTimeout(pageTimeoutMs) { withContext(Dispatchers.IO) { fetch(core) } }
                    }
                    fetched++
                    return
                } catch (e: CancellationException) {
                    // A timeout is a cancellation too; only ours is a skipped page.
                    if (e !is kotlinx.coroutines.TimeoutCancellationException) throw e
                    failed++
                    Log.warn("sync", "page timed out", f, mapOf("page" to id))
                    return
                } catch (e: CoreException.OtpRequired) {
                    if (attempt > 0 || !otp(e, f)) throw Stop()
                } catch (e: Throwable) {
                    failed++
                    Log.warn("sync", "page failed: ${e::class.simpleName}", f, mapOf("page" to id))
                    return
                }
            }
        }

        try {
            page("semesters") { it.fetchSemesters(f) }
            SemesterChoice.readCache(f)
            page("attendance") { it.fetchAttendance(f) }
            for (key in withContext(Dispatchers.IO) { core.mismatchedAttendanceDetails(f) }) page("history.$key") { it.fetchAttendanceDetail(f, key) }
            page("timetable") { it.fetchTimetable(f) }
            page("marks") { it.fetchMarks(f) }
            page("exams") { it.fetchExams(f) }
            page("calendar") { it.fetchCalendar(f) }
            SemesterChoice.selected?.let { semester -> page("grades") { it.fetchGrades(f, semester) } }
            page("grade_history") { it.fetchGradeHistory(f) }
            if (everything) {
                page("profile") { it.fetchProfile(f) }
                page("course_page") { it.fetchCoursePage(f) }
                page("general_outing") { it.fetchGeneralOuting(f) }
                page("weekend_outing") { it.fetchWeekendOuting(f) }
            }
        } catch (_: Stop) {
            Log.info("sync", "stopped at the otp", f, mapOf("fetched" to "$fetched", "failed" to "$failed"))
            return@flow Result(Outcome.StoppedAtOtp, fetched, failed)
        }
        Log.info("sync", "done", f, mapOf("fetched" to "$fetched", "failed" to "$failed"))
        Result(Outcome.Done, fetched, failed)
    }
}
