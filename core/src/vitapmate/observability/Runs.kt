package vitapmate.observability

import androidx.compose.runtime.Immutable

/**
 * The app's VTOP work as the student lives it: each run (opening a page, a pull, the background
 * sweep, a sign-in) as a short chain of steps, e.g. "Timetable", "Sign in → OTP → Timetable" or
 * "Timetable (not signed in) → Sign in → OTP → Timetable". [runsOf] is the one mapping from the
 * log's technical events to these steps:
 *
 * - a `core.<call>` span (Rust, closed with `ms` and `outcome`) that talked to VTOP is a step
 *   named after its page (`fetch_timetable` → "Timetable", `sign_in` → "Sign in");
 * - `session expired, signing in again` (core) ends that step as not signed in and starts
 *   "Sign in"; `rust.auth` stages end it (`login completed`, `restored session validation
 *   succeeded`), turn it into "OTP" (`security OTP verification is required`) or fail it;
 * - `rust.network` responses are the pages fetched in a step; none in an ok step: served cached;
 * - `route used` / `route failed` lines (Rust's route switch) give the run's [Run.route] and
 *   [Run.fallback];
 * - a later `*.submit_otp` / `*.resend_otp` / `otp.cancel` flow settles the waiting OTP step, and
 *   the refreshes it sets off (trigger `otp`, or opens right after a sign-in) join that run.
 */
@Immutable
data class Run(
    /** The first flow's id (for the Inspector). */
    val id: String,
    val flows: List<String>,
    val title: String,
    val trigger: RunTrigger,
    val startT: Long,
    val steps: List<RunStep>,
    /** How VTOP's answers came: "Direct", "Proxy", "Server" or "UDP"; null before any did. */
    val route: String? = null,
    /** "Proxy failed → Direct" when a route failed and the run fell back. */
    val fallback: String? = null,
) {
    val endT: Long? get() = if (running) null else steps.mapNotNull { it.endT }.maxOrNull()
    val running: Boolean get() = steps.any { it.state == StepState.Running }
    val waiting: Boolean get() = steps.lastOrNull()?.state == StepState.WaitingOtp
    val failed: Boolean get() = steps.lastOrNull()?.state == StepState.Failed
}

@Immutable
data class RunStep(
    val label: String,
    val state: StepState,
    val startT: Long,
    /** Null while it runs or waits. */
    val endT: Long?,
    /** A few words: why it failed or was skipped, or a note ("2 tries"). */
    val reason: String?,
    val pages: List<PageHit>,
    /** Steps of one kind in a row ("Class history" × 5) fold into one. */
    val count: Int = 1,
) {
    val ms: Long? get() = endT?.let { it - startT }
}

enum class StepState { Done, Running, WaitingOtp, Failed, Skipped, Cached }

/** One VTOP response: its page in words ("Attendance"), HTTP status and ms. */
@Immutable
data class PageHit(val name: String, val status: String, val ms: Long?)

enum class RunTrigger { Opened, Pulled, Retried, Automatic, Returned, AfterOtp, SignedIn, Tapped }

/**
 * Runs in [flows], newest first. A flow that never ended and started before [since] (the
 * process start) was cut off: its running steps show as interrupted.
 */
fun runsOf(flows: List<FlowSummary>, since: Long = 0): List<Run> {
    val runs = mutableListOf<MRun>()
    // Background syncs that finished: when, and how many changes they notified (joined below).
    val notified = mutableListOf<Pair<FlowSummary, String>>()
    for (f in flows.sortedBy { it.startT }) {
        // The prompt this flow answers: OTP steps that started waiting shortly before.
        fun waitingSteps() = runs.flatMap { r -> r.steps.filter { it.state == StepState.WaitingOtp && f.startT - it.startT in 0..OTP_WINDOW_MS }.map { r to it } }
        val name = f.name
        val end = f.lines.lastOrNull { it.src == "flow" && it.fields["ev"] in ENDS }?.t
        when {
            name.startsWith("forwarding.") -> Unit
            // The worker's own flow: a run of its own when it stopped before VTOP ("skipped: …"),
            // else only the count it notified, added to its pages' run.
            name == "sync.background" -> {
                val skipped = f.lines.firstOrNull { it.src == "sync" && it.msg.startsWith("skipped: ") }
                if (skipped != null) {
                    val step = MStep(BACKGROUND_SYNC, StepState.Skipped, f.startT).apply { end(StepState.Skipped, skipped.t, skipped.msg.removePrefix("skipped: ")) }
                    runs += MRun(f.id, BACKGROUND_SYNC, RunTrigger.Automatic, f.startT, listOf(step))
                }
                f.lines.firstOrNull { it.src == "sync" && it.msg == "background done" }?.fields?.get("notified")?.let { notified += f to it }
            }
            name.endsWith(".submit_otp") -> when (f.outcome) {
                null -> waitingSteps().forEach { (_, s) -> s.reason = "Checking the code" }
                "ok" -> waitingSteps().forEach { (r, s) ->
                    s.state = StepState.Done
                    s.endT = end
                    s.reason = null
                    r.otpAcceptedAt = end
                    if (r.trigger == RunTrigger.SignedIn) r.signedInAt = end
                }
                else -> waitingSteps().forEach { (_, s) -> s.reason = reasonOf(f.outcome) }
            }
            name.endsWith(".resend_otp") -> if (f.outcome == "ok") waitingSteps().forEach { (_, s) -> s.reason = "New code sent" }
            name == "otp.cancel" || name.endsWith(".cancel_otp") -> waitingSteps().forEach { (_, s) ->
                s.state = StepState.Skipped
                s.endT = f.startT
                s.reason = "Cancelled"
            }
            else -> {
                val steps = stepsOf(f, since)
                if (steps.isEmpty()) continue
                val title = titleOf(f, steps)
                val trigger = triggerOf(f)
                val host = when (trigger) {
                    RunTrigger.AfterOtp -> runs.filter { r -> r.otpAcceptedAt?.let { f.startT - it in 0..FOLLOW_OTP_MS } == true }
                        .let { c -> c.lastOrNull { it.title == title } ?: c.lastOrNull() }
                    RunTrigger.Opened -> runs.lastOrNull { r -> r.signedInAt?.let { f.startT - it in 0..FOLLOW_SIGN_IN_MS } == true }
                    else -> null
                }
                val run = host ?: MRun(f.id, title, trigger, f.startT, mutableListOf()).also { runs += it }
                run.steps += steps
                if (host != null) host.flows += f.id
                routeOf(f.lines)?.let { run.route = it }
                fallbackOf(f.lines)?.let { run.fallback = it }
                if (host == null) {
                    if (trigger == RunTrigger.SignedIn && f.outcome == "ok" && steps.none { it.state == StepState.WaitingOtp }) run.signedInAt = end
                }
            }
        }
    }
    for ((f, count) in notified) {
        val run = runs.lastOrNull { it.title == BACKGROUND_SYNC && it.startT >= f.startT } ?: continue
        val t = f.lines.last().t
        val label = when (count) { "0" -> "Nothing new"; "1" -> "1 change notified"; else -> "$count changes notified" }
        run.steps += MStep(label, StepState.Done, t).apply { end(StepState.Done, t) }
    }
    return runs.map { it.freeze() }.sortedByDescending { it.startT }
}

/**
 * An OTP step still waiting stays so only while the prompt is up ([pending]) in this process
 * (started after [since]); otherwise the code was never entered.
 */
fun List<Run>.settleOtp(pending: Boolean, since: Long): List<Run> = map { run ->
    if (run.steps.none { it.state == StepState.WaitingOtp && (!pending || it.startT < since) }) run
    else run.copy(steps = run.steps.map { s ->
        if (s.state == StepState.WaitingOtp && (!pending || s.startT < since)) s.copy(state = StepState.Skipped, reason = "Not entered") else s
    })
}

private const val OTP_WINDOW_MS = 15 * 60_000L
private const val FOLLOW_OTP_MS = 120_000L
private const val FOLLOW_SIGN_IN_MS = 20_000L
private const val SIGN_IN = "Sign in"
private const val BACKGROUND_SYNC = "Background sync"
private const val OTP = "OTP"
private const val NET = "rust.network"
private const val AUTH = "rust.auth"
private val ENDS = setOf("done", "failed", "cancelled")
private val SERVER_ROUTES = setOf("server", "udp")

private class MStep(val label: String, var state: StepState, val startT: Long) {
    var endT: Long? = null
    var reason: String? = null
    val pages = mutableListOf<PageHit>()
    var cached = false
    var retries = 0

    fun end(state: StepState, t: Long, reason: String? = null) {
        this.state = state
        endT = t
        if (reason != null) this.reason = reason
    }

    fun freeze() = RunStep(label, state, startT, endT, reason ?: retries.takeIf { it > 0 }?.let { "${it + 1} tries" }, pages.toList())
}

private class MRun(val id: String, val title: String, val trigger: RunTrigger, val startT: Long, steps: List<MStep>) {
    val flows = mutableListOf(id)
    val steps = steps.toMutableList()
    var route: String? = null
    var fallback: String? = null
    var otpAcceptedAt: Long? = null
    var signedInAt: Long? = null

    fun freeze(): Run {
        // "Class history" × 5: one step.
        val folded = mutableListOf<RunStep>()
        for (s in steps.map { it.freeze() }) {
            val last = folded.lastOrNull()
            if (last != null && last.label == s.label && last.state == s.state && s.state in FOLDS) {
                folded[folded.lastIndex] = last.copy(endT = s.endT ?: last.endT, pages = last.pages + s.pages, count = last.count + 1)
            } else folded += s
        }
        return Run(id, flows.toList(), title, trigger, startT, folded, route, fallback)
    }
}

private val FOLDS = setOf(StepState.Done, StepState.Cached, StepState.Running)

/** A flow's steps: its lines cut at each core span close; spans without VTOP work are left out. */
private fun stepsOf(f: FlowSummary, since: Long): List<MStep> {
    val out = mutableListOf<MStep>()
    var seg = mutableListOf<LogLine>()
    // The last local call (`cached_marks` before the sweep's `fetch_marks`): names a running step.
    var hint: String? = null
    val live = f.outcome == null && f.startT >= since
    fun flush(close: LogLine?) {
        val call = close?.src?.removePrefix("core.")
        val ui = seg.firstOrNull { it.src.startsWith("ui.") && it.fields["state"] in STARTS }
        // Restoring the saved session (core init) logs auth lines but asks VTOP nothing.
        val vtop = seg.any { it.src == NET || (it.src == AUTH && "login requested" in it.msg) } || call?.startsWith("fetch_") == true || call == "sign_in" ||
            (close == null && live && ui != null)
        when {
            call != null && "forward" in call -> Unit
            vtop -> out += segment(seg, call, labelOf(call, ui?.src?.removePrefix("ui."), hint, f))
            call != null -> hint = call
        }
        seg = mutableListOf()
    }
    for (line in f.lines) {
        if (line.src == "flow") continue
        seg += line
        if (line.src.startsWith("core.") && line.msg == "done" && line.outcome != null) flush(line)
    }
    if (seg.isNotEmpty()) flush(null)
    if (!live) {
        val end = f.lines.lastOrNull { it.src == "flow" && it.fields["ev"] in ENDS }
        for (s in out.filter { it.state == StepState.Running }) {
            val t = end?.t ?: f.lines.last().t
            when (val o = f.outcome) {
                null -> s.end(StepState.Skipped, t, "Interrupted")
                "ok" -> s.end(if (s.pages.isEmpty() && s.label != SIGN_IN) StepState.Cached else StepState.Done, t)
                "cancelled" -> s.end(StepState.Skipped, t, "Stopped")
                else -> s.end(StepState.Failed, t, reasonOf(o))
            }
        }
    }
    return out
}

/** One core call's lines (its span close last, none while it runs) as steps. */
private fun segment(lines: List<LogLine>, call: String?, label: String): List<MStep> {
    val out = mutableListOf<MStep>()
    val page = label != SIGN_IN
    val close = lines.last().takeIf { it.src.startsWith("core.") && it.msg == "done" && it.outcome != null }
    // A Server or UDP call logs nothing until its answer is back, so its first line is not its start: the span's is.
    val start = minOf(lines.first().t, close?.ms?.let { close.t - it } ?: Long.MAX_VALUE)
    // The UDP round trip as Rust timed it.
    var udpMs: Long? = null
    var cur = MStep(label, StepState.Running, start).also { out += it }
    fun open(l: String, state: StepState, t: Long) = MStep(l, state, t).also { out += it; cur = it }
    for (line in lines) {
        val m = line.msg
        when {
            line.src.startsWith("core.") && m.startsWith("session expired, signing in again") -> {
                cur.end(StepState.Skipped, line.t, "Not signed in")
                open(SIGN_IN, StepState.Running, line.t)
            }
            line.src.startsWith("core.") && m.startsWith("session expired; the saved password was rejected") -> {
                cur.end(StepState.Skipped, line.t, "Not signed in")
                open(SIGN_IN, StepState.Running, line.t).end(StepState.Failed, line.t, "Password rejected")
            }
            m == "attendance detail just fetched" -> cur.cached = true
            line.src == AUTH -> when {
                "security OTP verification is required" in m -> {
                    if (cur.label == SIGN_IN) cur.end(StepState.Done, line.t) else cur.end(StepState.Skipped, line.t, "Not signed in")
                    open(OTP, StepState.WaitingOtp, line.t)
                }
                ("login completed successfully" in m || "restored session validation succeeded" in m) && cur.label == SIGN_IN && cur.state == StepState.Running -> {
                    cur.end(StepState.Done, line.t)
                    if (call != "sign_in") open(label, StepState.Running, line.t)
                }
                "invalid credentials" in m && cur.label == SIGN_IN -> cur.end(StepState.Failed, line.t, "Password rejected")
                "captcha answer was rejected" in m -> cur.retries++
            }
            line.src == NET && m.startsWith("response ") -> hitOf(m)?.let { cur.pages += it }
            // Server and UDP fetch VTOP on the server, so the phone logs no VTOP hits: the route is the hit.
            m == "udp fetch" -> udpMs = line.ms
            m == "route used" && line.fields["route"]?.lowercase() in SERVER_ROUTES ->
                cur.pages += PageHit(routeName(line.fields.getValue("route")), "200", udpMs ?: (line.t - cur.startT))
        }
    }
    if (close == null) return out
    when (val o = close.outcome) {
        "ok" -> if (cur.state == StepState.Running) {
            cur.end(if (page && (cur.cached || cur.pages.isEmpty())) StepState.Cached else StepState.Done, close.t)
        }
        "OtpRequired" -> if (cur.state != StepState.WaitingOtp) {
            if (cur.state == StepState.Running) cur.end(StepState.Skipped, close.t, "Not signed in")
            open(OTP, StepState.WaitingOtp, close.t)
        }
        else -> if (cur.state == StepState.Running) cur.end(StepState.Failed, close.t, reasonOf(o))
    }
    return out
}

private val STARTS = setOf("refreshing", "fetching")

/** The route of the flow's last `route used` line, in words. */
fun routeOf(lines: List<LogLine>): String? = lines.lastOrNull { it.msg == "route used" }?.fields?.get("route")?.let(::routeName)

/** "Proxy failed → Direct": the flow's last failed route and the one used after it. */
fun fallbackOf(lines: List<LogLine>): String? {
    val i = lines.indexOfLast { it.msg.startsWith("route failed") }.takeIf { it >= 0 } ?: return null
    val failed = lines[i].fields["route"]?.let(::routeName) ?: return null
    return "$failed failed → ${routeOf(lines.drop(i + 1)) ?: "Direct"}"
}

private fun routeName(route: String) = if (route.equals("udp", true)) "UDP" else route.replaceFirstChar { it.uppercase() }

/** "response vtop.vitap.ac.in/vtop/processViewStudentAttendance 200 158ms" (a size may come before the ms). */
private fun hitOf(msg: String): PageHit? {
    val parts = msg.split(' ')
    if (parts.size < 3) return null
    val path = parts[1].substringBefore('?').substringAfter('/')
    return PageHit(vtopPage(path), parts[2], parts.last().takeIf { it.endsWith("ms") }?.removeSuffix("ms")?.toLongOrNull())
}

/** A VTOP path in words ("vtop/processViewTimeTable" → "Timetable"); unknown ones by their last part. */
private fun vtopPage(path: String): String =
    VTOP_PAGES.firstOrNull { (key, _) -> path.contains(key, ignoreCase = true) }?.second
        ?: path.trimEnd('/').substringAfterLast('/').ifEmpty { "VTOP" }

// First match wins: "login/error" before "login".
private val VTOP_PAGES = listOf(
    "open/page" to "VTOP",
    "prelogin/setup" to "Sign-in page",
    "login/error" to "Sign-in error",
    "validateSecurityOtp" to "OTP check",
    "resendSecurityOtp" to "New OTP",
    "vtop/login" to "Sign-in",
    "vtop/content" to "Home",
    "processViewStudentAttendance" to "Attendance",
    "processViewAttendanceDetail" to "Class history",
    "processViewTimeTable" to "Timetable",
    "StudentTimeTable" to "Timetable page",
    "processViewCalendar" to "Calendar",
    "getDateForSemesterPreview" to "Calendar months",
    "getSlotIdForCoursePage" to "Course slots",
    "getCourseForCoursePage" to "Courses",
    "processViewStudentCourseDetail" to "Course detail",
    "StudentCoursePage" to "Course pages",
    "CoursePlanExcelDownload" to "Course plan",
    "downloadPdf" to "Course file",
    "doStudentMarkView" to "Marks",
    "doSearchExamScheduleForStudent" to "Exam schedule",
    "doStudentGradeView" to "Grades",
    "getGradeViewDetails" to "Grade details",
    "StudentGradeHistory" to "Grade history",
    "getStudBioHistory" to "Biometric",
    "StudentGeneralOuting" to "Outings",
    "StudentWeekendOuting" to "Weekend outings",
    "saveGeneralOutingForm" to "Outing form",
    "saveOutingForm" to "Outing form",
    "deleteGeneralOutingInfo" to "Outing cancel",
    "deleteBookingInfo" to "Outing cancel",
    "downloadLeavePass" to "Outing pass",
    "downloadOutingForm" to "Outing pass",
    "StudentProfileAllView" to "Profile",
    "viewProctorDetails" to "Mentor",
    "viewMessagesSendByProctor" to "Mentor messages",
)

private fun labelOf(call: String?, ui: String?, hint: String?, f: FlowSummary): String = when {
    call == "sign_in" -> SIGN_IN
    call != null -> pageLabel(call.removePrefix("fetch_").removePrefix("download_"))
    ui != null -> pageLabel(ui)
    hint != null && hint.startsWith("cached_") -> pageLabel(hint.removePrefix("cached_"))
    else -> titleOf(f, emptyList())
}

private fun titleOf(f: FlowSummary, steps: List<MStep>): String {
    val name = f.name
    return when {
        name == "login.sign_in" -> SIGN_IN
        name == "sync.stale" -> "Background refresh"
        name == "sync.all" -> "Fetch all"
        name.startsWith("sync.background") -> BACKGROUND_SYNC
        name.endsWith(".refresh") -> pageLabel(name.removeSuffix(".refresh"))
        name.endsWith(".calendar") -> "Calendar"
        name.endsWith(".exams") -> "Exams"
        name.endsWith(".histories") -> "Class history"
        else -> steps.firstOrNull { it.label != SIGN_IN && it.label != OTP }?.label ?: pageLabel(name.substringBefore('.'))
    }
}

private fun triggerOf(f: FlowSummary): RunTrigger {
    val trigger = f.lines.firstNotNullOfOrNull { l -> l.fields["trigger"]?.takeIf { l.src.startsWith("ui.") } }
    return when {
        f.name == "login.sign_in" -> RunTrigger.SignedIn
        f.name == "sync.stale" -> RunTrigger.Automatic
        trigger in setOf("pull", "user", "button") -> RunTrigger.Pulled
        trigger == "retry" -> RunTrigger.Retried
        trigger in setOf("open", "rejoin", "choose", "settings") -> RunTrigger.Opened
        trigger == "resume" -> RunTrigger.Returned
        trigger == "otp" -> RunTrigger.AfterOtp
        trigger != null || f.name.endsWith(".refresh") || f.name.endsWith(".calendar") || f.name.endsWith(".exams") || f.name.endsWith(".histories") -> RunTrigger.Automatic
        else -> RunTrigger.Tapped
    }
}

/** A failure's kind (Rust's `CoreError` kind, or the Kotlin exception's class) in a few words. */
private fun reasonOf(outcome: String?): String = when (outcome) {
    "Offline" -> "No internet"
    "VtopUnavailable" -> "VTOP is down"
    "InvalidCredentials" -> "Password rejected"
    "NotSignedIn" -> "Signed out"
    "OtpRejected" -> "Code not accepted"
    "OtpRequired" -> "Needs the OTP"
    "SemesterNeeded" -> "No semester picked"
    "cancelled" -> "Stopped"
    else -> "Something went wrong"
}

private fun pageLabel(id: String): String = PAGES[id] ?: id.replace('_', ' ').replaceFirstChar { it.uppercase() }

private val PAGES = mapOf(
    "attendance_detail" to "Class history",
    "course_page" to "Course pages",
    "coursepage" to "Course pages",
    "course_view" to "Course",
    "coursepage_course" to "Course",
    "grade_history" to "Grade history",
    "grade_details" to "Grade details",
    "biometric_range" to "Biometric",
    "general_outing" to "Outings",
    "weekend_outing" to "Weekend outing",
    "outing" to "Outings",
    "semesters" to "Semesters",
)
