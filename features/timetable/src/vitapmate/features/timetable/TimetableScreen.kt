package vitapmate.features.timetable

import androidx.compose.animation.Crossfade
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarX
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import vitapmate.core.AttendanceCourse
import vitapmate.core.ClassDay
import vitapmate.core.DayClass
import vitapmate.core.Timetable
import vitapmate.core.TimetableDay
import vitapmate.data.Activity
import vitapmate.data.AppCore
import vitapmate.data.CalendarRefresh
import vitapmate.data.ExamDayRefresh
import vitapmate.data.SemesterChoice
import vitapmate.data.Settings
import vitapmate.data.rememberPage
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.EmptyView
import vitapmate.ui.PageScaffold
import vitapmate.ui.is24Hour
import vitapmate.ui.isoWeekday
import vitapmate.ui.mondayOf
import vitapmate.ui.rememberNow

/** Days read with the timetable: this week, next week (Sunday's grid) and four more to look ahead. */
private const val WINDOW = 42

/** What the screen draws besides the timetable, from Rust's saved copies (no VTOP request). */
private class Extra(val days: List<TimetableDay>, val attendance: List<AttendanceCourse>)

/**
 * The timetable page (log src `ui.timetable`, flows `timetable.*`, state dump key `timetable`):
 * [PageScaffold] draws loading, failure, the banner and the age; this draws the agenda or the
 * weekly grid and the class sheet.
 */
@Composable
fun TimetableScreen() {
    val page = rememberPage("timetable", { c, f -> c.cachedTimetable(f) }, { c, f -> c.fetchTimetable(f) }, { it.updatedAt }) { it.slots.size }
    PageScaffold(
        page.state,
        onRefresh = { page.refresh("user") },
        isEmpty = { it.slots.isEmpty() },
        empty = {
            Box(Modifier.testTag("timetable.empty")) {
                EmptyView(
                    Lucide.CalendarX,
                    "No timetable yet",
                    message = "VTOP has no classes for this semester yet. Once your courses are registered they show up here. Pull down to check again.",
                )
            }
        },
        modifier = Modifier.testTag("screen.timetable"),
        loading = { TimetableSkeleton() },
    ) { TimetableContent(it) }
}

@Composable
private fun TimetableContent(tt: Timetable) {
    val now = rememberNow()
    val today by remember { derivedStateOf { istDay(now.value) } }
    val semester = SemesterChoice.selected
    // Bumped when the academic calendar arrives, so the days are read again with it.
    var calendars by remember { mutableIntStateOf(0) }
    LaunchedEffect(tt.updatedAt, semester) { if (CalendarRefresh.ifOld(tt.semesterId, "timetable")) calendars++ }
    // On a day with one of your papers, the schedule again (a moved venue, seat or time).
    LaunchedEffect(tt.semesterId, today) { if (ExamDayRefresh.ifExamToday(tt.semesterId, "timetable")) calendars++ }
    // Bumped when class histories arrive, so held classes show what VTOP posted.
    var histories by remember { mutableIntStateOf(0) }
    LaunchedEffect(tt.updatedAt, semester, today, calendars) {
        if (Settings.autoRefresh && fetchStaleHistories(tt.semesterId, mondayOf(today)) > 0) histories++
    }
    val extra by produceState<Extra?>(null, tt, today, semester, calendars, histories) {
        value = flow("timetable.days") { id -> withContext(Dispatchers.IO) { loadExtra(id, tt, today) } }
    }
    val h24 = is24Hour()
    val mode = timetableViewMode()
    // One state for the screen's life (not keyed on the semester: a recreated state leaves
    // callbacks held by the pager writing to the old one); a semester switch goes back to today.
    var selectedDay by rememberSaveable { mutableIntStateOf(isoWeekday(today)) }
    var daySemester by rememberSaveable { mutableStateOf(semester) }
    LaunchedEffect(semester) {
        if (semester != daySemester) {
            if (daySemester != null) selectedDay = isoWeekday(today)
            daySemester = semester
        }
    }
    var sheet by remember { mutableStateOf<DayClass?>(null) }
    val scope = rememberCoroutineScope()
    val ext = extra ?: return TimetableSkeleton()

    LaunchedEffect(mode, selectedDay, ext) {
        StateRegistry.report(
            "timetable",
            mapOf(
                "view" to mode.name.lowercase(),
                "selectedDay" to selectedDay,
                "semester" to tt.semesterId,
                "slots" to tt.slots.size,
                "attendanceCourses" to ext.attendance.size,
                "zoneNote" to (zoneNote(System.currentTimeMillis()) != null),
                "sheet" to sheet?.let { classTag(it.slot) },
            ),
        )
    }
    DisposableEffect(Unit) { onDispose { StateRegistry.remove("timetable") } }

    fun select(day: Int, how: String) {
        selectedDay = day
        scope.launch { flow("timetable.day") { id -> Log.info("ui.timetable", "day $day", id, mapOf("day" to "$day", "how" to how)) } }
    }
    fun open(c: DayClass, from: String) {
        sheet = c
        scope.launch {
            flow("timetable.class") { id ->
                Log.info("ui.timetable", "open class", id, mapOf("code" to c.slot.code, "tag" to classTag(c.slot), "from" to from, "attendance" to "${ext.attendance.forSlot(c.slot) != null}"))
            }
        }
    }

    Column(Modifier.fillMaxSize()) {
        Crossfade(mode, Modifier.weight(1f), label = "timetable-view") { m ->
            if (m == TimetableViewMode.Agenda) {
                AgendaView(ext.days, ext.attendance, selectedDay, ::select, ::open, now, h24, Modifier.testTag("timetable.agenda"))
            } else {
                val classDays = remember(tt.slots) { tt.slots.map { it.day + 1 }.distinct().sorted() }
                WeeklyGrid(
                    classDays,
                    tt.slots,
                    window = ext.days,
                    now = now,
                    h24 = h24,
                    // The class as it is this week (a Lab FAT shows as one).
                    onOpen = { s -> open(ext.days.getOrNull(s.day)?.classes?.firstOrNull { it.slot == s } ?: DayClass(s, ClassDay.HELD, false, null), "weekly") },
                    modifier = Modifier.testTag("timetable.weekly"),
                )
            }
        }
    }
    sheet?.let { c ->
        val labFat = c.state == ClassDay.LAB_FAT
        ClassSheet(c.slot, if (labFat) null else ext.attendance.forSlot(c.slot), labFat, h24, onDismiss = { sheet = null })
    }
}

/** The days and attendance from Rust's cache; a failure falls back to the bare timetable (every class held, no calendar). */
private suspend fun loadExtra(flow: String, tt: Timetable, today: Int): Extra {
    val core = AppCore.await()
    val from = mondayOf(today)
    val days = try {
        core.timetableDays(flow, from, WINDOW).takeIf { it.size >= 14 } ?: error("only some days")
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        Log.warn("ui.timetable", "agenda days unavailable, timetable only", flow, mapOf("error" to e.toString().take(200)))
        (0 until WINDOW).map { i ->
            val d = from + i
            TimetableDay(d, tt.slots.filter { it.day + 1 == isoWeekday(d) }.map { DayClass(it, ClassDay.HELD, false, null) }, null, null, true, false, emptyList(), 0, null)
        }
    }
    val attendance = try {
        core.cachedAttendance(flow)?.courses.orEmpty()
    } catch (e: Exception) {
        Log.warn("ui.timetable", "attendance cache unreadable", flow, mapOf("error" to e.toString().take(200)))
        emptyList()
    }
    Log.info(
        "ui.timetable",
        "days",
        flow,
        mapOf(
            "from" to "$from",
            "days" to "${days.size}",
            "today_classes" to "${days.getOrNull(today - from)?.let(::meeting)?.size ?: 0}",
            "attendance" to "${attendance.size}",
            "zone_note" to "${zoneNote(System.currentTimeMillis()) != null}",
            "clock_shift_min" to "$clockShiftMin",
        ),
    )
    return Extra(days, attendance)
}


/** One history sync at a time, and each course tried at most once per [HISTORY_RETRY_MS]. */
private val historyLock = Mutex()
private val historyTries = mutableMapOf<String, Long>()
private const val HISTORY_RETRY_MS = 15 * 60_000L

/**
 * Fetches, one course at a time in the background (flow `timetable.histories`), the class history
 * of each course with a class held this window whose saved history is missing or behind VTOP's
 * summary (Rust decides from the cache), so held classes show Present / Absent / On duty. Returns
 * how many arrived; failures are logged and skipped. The screen never waits for it.
 */
private suspend fun fetchStaleHistories(semester: String, from: Int): Int = historyLock.withLock {
    flow("timetable.histories") { id ->
        withContext(Dispatchers.IO) {
            val core = AppCore.await()
            val now = System.currentTimeMillis()
            val due = core.staleAttendanceDetails(id, from, WINDOW).filter { key -> historyTries["$semester/$key"]?.let { now - it >= HISTORY_RETRY_MS } ?: true }
            if (due.isEmpty()) return@withContext 0
            Log.info("ui.timetable", "histories to fetch", id, mapOf("courses" to "${due.size}"))
            Activity.track("Refreshing class history", id) {
                due.count { key ->
                    historyTries["$semester/$key"] = System.currentTimeMillis()
                    try {
                        core.fetchAttendanceDetail(id, key)
                        true
                    } catch (e: CancellationException) {
                        historyTries.remove("$semester/$key")
                        throw e
                    } catch (e: Exception) {
                        Log.warn("ui.timetable", "history not fetched", id, mapOf("course" to key, "error" to (e::class.simpleName ?: "Exception")))
                        false
                    }
                }
            }.also { Log.info("ui.timetable", "histories fetched", id, mapOf("fetched" to "$it", "due" to "${due.size}")) }
        }
    }
}
