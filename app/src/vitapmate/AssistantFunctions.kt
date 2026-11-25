package vitapmate

import androidx.annotation.RequiresApi
import androidx.appfunctions.AppFunctionDeclaration
import androidx.appfunctions.AppFunctionInvalidArgumentException
import androidx.appfunctions.AppFunctionPermissionRequiredException
import androidx.appfunctions.AppFunctionSerializable
import androidx.appfunctions.AppFunctionService
import androidx.appfunctions.AppFunctionServiceEntryPoint
import androidx.compose.runtime.Composable
import androidx.compose.runtime.rememberCoroutineScope
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sparkles
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.core.ClassKind
import vitapmate.core.ClassSlot
import vitapmate.core.Core
import vitapmate.core.OutingKind
import vitapmate.data.AppCore
import vitapmate.data.Docs
import vitapmate.data.Meal
import vitapmate.data.MessTimings
import vitapmate.data.istDay
import vitapmate.data.istMinute
import vitapmate.data.messMenuYear
import vitapmate.data.parseMessMenu
import vitapmate.data.readSpreadsheet
import vitapmate.data.Settings
import vitapmate.features.timetable.meeting
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.formatDay
import vitapmate.ui.rememberHaptics

/** A class on the student's timetable. Dates and times are India time (IST). */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantClass(
    /** The course's name. */
    val course: String,
    /** The course code, like "SWE3004". */
    val code: String,
    /** The date, as YYYY-MM-DD. */
    val date: String,
    /** When it starts, as HH:MM on a 24-hour clock. */
    val start: String,
    /** When it ends, as HH:MM on a 24-hour clock. */
    val end: String,
    /** The room and block, like "214 CB"; empty when VTOP gives none. */
    val room: String,
    /** The teacher's name. */
    val faculty: String,
    /** True for a lab session, false for a theory class. */
    val lab: Boolean,
)

/** The student's attendance in one course; theory and lab are counted apart. */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantAttendance(
    /** The course's name. */
    val course: String,
    /** The course code, like "SWE3004". */
    val code: String,
    /** True for the lab part, false for theory. */
    val lab: Boolean,
    /** Classes attended. */
    val attended: Int,
    /** Classes held. */
    val total: Int,
    /** The percentage as VTOP shows it; below 75 the student can be debarred from the next exam. */
    val percent: Double,
)

/**
 * One course's attendance outlook: how many classes can still be missed, or must be attended, to
 * finish at 75% when attendance is next checked (before the next exam, else at the semester's end).
 * Below 75% the student can be debarred from that exam.
 */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantAttendancePlan(
    /** The course's name; empty when the saved attendance doesn't list it. */
    val course: String,
    /** The course code, like "SWE3004". */
    val code: String,
    /** True for the lab part, false for theory (they are counted apart). */
    val lab: Boolean,
    /** The exam attendance is checked for, like "CAT-II" or "FAT"; empty for the semester's end. */
    val until: String,
    /** The day counting stops before (the exam's first day, or the day after the last class day), as YYYY-MM-DD in India time. */
    val untilDate: String,
    /** Classes attended, as VTOP's percentage counts them. */
    val attended: Int,
    /** Classes held, as VTOP's percentage counts them. */
    val held: Int,
    /** Classes still to come before [untilDate], by the timetable and academic calendar. */
    val upcoming: Int,
    /** Classes held but not posted on VTOP yet. */
    val unposted: Int,
    /** Of the upcoming and unposted classes, how many can be missed and still finish at 75%. */
    val canMiss: Int,
    /** How many of them must be attended to finish at 75%; null when even attending all of them is not enough. */
    val mustAttend: Int?,
    /** The percentage if every remaining class is attended. */
    val bestPercent: Double,
    /** The app's advice on the current figure, like "Can skip 3", "Attend 2 labs" or "Don't skip"; empty when unknown. */
    val advice: String,
    /** True when VTOP has debarred the student in this course. */
    val debarred: Boolean,
)

/** One exam paper. Dates and times are India time (IST). */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantExam(
    /** The exam, like "CAT-I", "CAT-II" or "FAT". */
    val exam: String,
    /** The course's name. */
    val course: String,
    /** The course code, like "SWE3004". */
    val code: String,
    /** True for a lab exam, false for theory. */
    val lab: Boolean,
    /** The date, as YYYY-MM-DD; null until VTOP dates it. */
    val date: String?,
    /** When it starts, as HH:MM on a 24-hour clock; null when not given. */
    val start: String?,
    /** When it ends, as HH:MM on a 24-hour clock; null when not given. */
    val end: String?,
    /** When to report at the venue, as HH:MM on a 24-hour clock; null when not given. */
    val reporting: String?,
    /** "FN" (forenoon) or "AN" (afternoon); empty when not given. */
    val session: String,
    /** The venue, like "AB1-204"; empty until VTOP gives it. */
    val venue: String,
    /** The seat, like "R6 · C6"; empty until VTOP gives it. */
    val seat: String,
)

/** One assessment of a course, like "CAT-I" or "Quiz-1". */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantAssessment(
    /** VTOP's title. */
    val title: String,
    /** The mark scored; null until posted. */
    val scored: Double?,
    /** The maximum mark. */
    val max: Double,
    /** How much of the course's 100 it counts for; null when VTOP gives none. */
    val weightage: Double?,
    /** What it adds to the course's total out of 100; null until posted. */
    val weighted: Double?,
    /** VTOP's status, like "Present" or "Absent". */
    val status: String,
)

/** The student's marks in one course this semester. */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantMarks(
    /** The course's name. */
    val course: String,
    /** The course code, like "SWE3004". */
    val code: String,
    /** True for a lab or project part, false for theory. */
    val lab: Boolean,
    /** Weighted marks scored so far, out of [maxWeighted]. */
    val weighted: Double,
    /** The weightage of what was assessed so far, out of the course's 100. */
    val maxWeighted: Double,
    /** [weighted] as a share of [maxWeighted], 0 to 100; null before anything is assessed. */
    val percent: Double?,
    /** The assessments, in VTOP's order. */
    val assessments: List<AssistantAssessment>,
)

/** One course in the grade history. */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantGradeCourse(
    /** The course code, like "SWE3004". */
    val code: String,
    /** The course's title. */
    val title: String,
    /** Its credits; null when VTOP gives none. */
    val credits: Double?,
    /** The grade: S, A, B, C, D, E or F (grade points 10 to 5, F 0), or P/N for pass/fail courses. */
    val grade: String,
)

/** One exam session (semester) of the grade history. */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantGradeSemester(
    /** The session, like "Jan 2025", or "Other". */
    val session: String,
    /** The session's credit-weighted GPA out of 10; null when it has no graded courses. */
    val gpa: Double?,
    /** Its courses. */
    val courses: List<AssistantGradeCourse>,
)

/** The student's grade history across semesters. */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantGrades(
    /** The CGPA out of 10, as VTOP gives it, like "9.12". */
    val cgpa: String,
    /** Credits earned, like "120.5". */
    val creditsEarned: String,
    /** Credits registered. */
    val creditsRegistered: String,
    /** Each exam session, in VTOP's order. */
    val semesters: List<AssistantGradeSemester>,
)

/** One outing request on VTOP. */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantOuting(
    /** True for a weekend outing, false for a general one. */
    val weekend: Boolean,
    /** Where to. */
    val place: String,
    /** Why. */
    val purpose: String,
    /** "pending", "approved", "rejected" or "other". */
    val standing: String,
    /** VTOP's status in its own words, like "Waiting for Mentor's Approval". */
    val status: String,
    /** The trip's times as VTOP shows them, like "Out: Sat 14 Nov, 9:00 AM". */
    val times: List<String>,
    /** "OUT NOW", "TODAY", "TOMORROW" or "IN 3 DAYS" for a trip not over and not rejected; null otherwise. */
    val upcoming: String?,
)

/** One day the academic calendar lists. */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantCalendarDay(
    /** The date, as YYYY-MM-DD in India time. */
    val date: String,
    /**
     * "exam" (no classes), "lab_fat" (lab exams in that day's lab slots, theory as usual),
     * "holiday", "no_classes", "special" or "classes".
     */
    val mark: String,
    /** Like "Deepavali", "CAT-II" or "No instructional day". */
    val title: String,
)

/** One dish on the mess menu. */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantDish(
    /** The dish, like "Paneer Butter Masala". */
    val name: String,
    /** True for a non-veg dish. */
    val nonVeg: Boolean,
)

/** One meal of the hostel mess. Times are India time (IST). */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantMeal(
    /** "Breakfast", "Lunch", "Snacks" or "Dinner". */
    val meal: String,
    /** When serving starts, as HH:MM on a 24-hour clock. */
    val start: String,
    /** When serving ends, as HH:MM on a 24-hour clock. */
    val end: String,
    /** What's served; empty when the menu lists nothing for it. */
    val dishes: List<AssistantDish>,
)

/** A day's hostel mess menu. */
@AppFunctionSerializable(isDescribedByKDoc = true)
data class AssistantMessMenu(
    /** The date, as YYYY-MM-DD in India time. */
    val date: String,
    /** The menu's sheet the student picked, like "Veg & Non-Veg" or "Special". */
    val sheet: String,
    /** Breakfast, lunch, snacks and dinner, in that order. */
    val meals: List<AssistantMeal>,
)

/**
 * App functions for Android's assistant (Android 16+, AppFunctions): the timetable, attendance,
 * exams, marks, grades, outings, academic calendar and mess menu the app has saved, read only and never from the network. Off until the student turns on
 * Settings → Assistant. The compiler generates `VitapmateFunctions` (the service in the manifest)
 * and `assets/vitapmate_functions.xml`. Flows `assistant.*`.
 */
@RequiresApi(36)
@AppFunctionServiceEntryPoint(serviceName = "VitapmateFunctions", appFunctionXmlFileName = "vitapmate_functions")
abstract class AssistantFunctions : AppFunctionService() {
    /**
     * The student's class that is on now, or else their next one in the coming week, from their
     * VIT-AP timetable. Null when there is none.
     */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getNextClass(): AssistantClass? = read("next_class") { c, f ->
        val ms = System.currentTimeMillis()
        val today = istDay(ms)
        c.timetableDays(f, today, 8).firstNotNullOfOrNull { d ->
            meeting(d).map { it.slot }.sortedBy { it.start }.firstOrNull { d.day > today || it.end > istMinute(ms) }?.let { info(it, d.day) }
        }
    }

    /**
     * The student's classes on one day of their VIT-AP timetable, in start order. Empty on a
     * holiday or a day without classes.
     *
     * @param daysFromToday 0 for today, 1 for tomorrow, up to 7.
     */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getClasses(daysFromToday: Int): List<AssistantClass> {
        if (daysFromToday !in 0..7) throw AppFunctionInvalidArgumentException("daysFromToday must be from 0 to 7")
        return read("classes") { c, f ->
            val day = istDay(System.currentTimeMillis()) + daysFromToday
            c.timetableDays(f, day, 1).firstOrNull()?.let { d -> meeting(d).map { it.slot }.sortedBy { it.start }.map { info(it, d.day) } }.orEmpty()
        }
    }

    /** The student's attendance in every course of the current semester, as VTOP last showed it. */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getAttendance(): List<AssistantAttendance> = read("attendance") { c, f ->
        c.cachedAttendance(f)?.courses.orEmpty().map { AssistantAttendance(it.name, it.code, it.kind == ClassKind.LAB, it.attended, it.total, it.percent) }
    }

    /**
     * Per course, how many classes the student can still miss, or must attend, to finish at 75%
     * attendance when it is next checked: before the next exam (below 75% then debars them from it),
     * else at the semester's end. Empty when the app has no academic calendar saved.
     */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getAttendancePlan(): List<AssistantAttendancePlan> = read("attendance_plan") { c, f ->
        val courses = c.cachedAttendance(f)?.courses.orEmpty().associateBy { it.key }
        c.attendanceOutlook(f).map { o ->
            val a = courses[o.key]
            val p = o.plan
            AssistantAttendancePlan(
                a?.name.orEmpty(), a?.code ?: o.key.substringBefore('.'), o.key.endsWith(".lab"), p.until.orEmpty(), formatDay(p.untilDay, "yyyy-MM-dd"),
                p.attended, p.held, p.upcoming, p.unposted, p.canMiss, p.mustAttend, p.bestPercent, a?.advice.orEmpty(), a?.debarred == true,
            )
        }
    }

    /**
     * The student's exam papers this semester that are still to come (or not dated yet), soonest
     * first, in India time. Empty when none are scheduled.
     *
     * @param includePast Also the papers already over in the exam that is on now or next.
     */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getExams(includePast: Boolean): List<AssistantExam> = read("exams") { c, f ->
        val ms = System.currentTimeMillis()
        val today = istDay(ms)
        val now = istMinute(ms)
        val exams = c.cachedExams(f)
        exams?.groups.orEmpty().flatMapIndexed { i, g ->
            g.papers.filter { p ->
                val d = p.day
                d == null || d > today || (d == today && (p.end ?: p.start ?: 1440) > now) || (includePast && i == exams?.opens)
            }.map { p ->
                AssistantExam(
                    g.name, p.name, p.code, p.kind == ClassKind.LAB, p.day?.let { formatDay(it, "yyyy-MM-dd") },
                    p.start?.let { hhmm(it) }, p.end?.let { hhmm(it) }, p.reporting?.let { hhmm(it) }, p.session.orEmpty(), p.venue.orEmpty(), p.seat.orEmpty(),
                )
            }
        }
    }

    /** The student's marks this semester per course, with each assessment and the weighted total so far (out of 100). */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getMarks(): List<AssistantMarks> = read("marks") { c, f ->
        c.cachedMarks(f)?.courses.orEmpty().map { m ->
            AssistantMarks(
                m.name, m.code, m.kind == ClassKind.LAB, m.weighted, m.maxWeighted, m.percent,
                m.assessments.map { AssistantAssessment(it.title, it.scored, it.max, it.weightage, it.weighted, it.status) },
            )
        }
    }

    /** The student's CGPA, credits and each semester's GPA and course grades. Null when the app has no grade history saved. */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getGrades(): AssistantGrades? = read("grades") { c, f ->
        c.cachedGradeHistory(f)?.let { g ->
            AssistantGrades(
                g.cgpa, g.creditsEarned, g.creditsRegistered,
                g.sessions.map { s -> AssistantGradeSemester(s.label, s.gpa, s.courses.map { AssistantGradeCourse(it.code, it.title, it.credits, it.grade) }) },
            )
        }
    }

    /** The student's general and weekend outing requests on VTOP with their status, as last fetched. Empty when there are none. */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getOutings(): List<AssistantOuting> = read("outings") { c, f ->
        (c.cachedGeneralOuting(f)?.requests.orEmpty() + c.cachedWeekendOuting(f)?.requests.orEmpty()).map { r ->
            AssistantOuting(
                r.kind == OutingKind.WEEKEND, r.place, r.purpose, r.standing.name.lowercase(), r.status,
                r.`when`.map { w -> listOfNotNull(w.label, w.text).joinToString(": ") }, r.highlight,
            )
        }
    }

    /**
     * The days VIT-AP's academic calendar lists (holidays, exams, Lab FATs, days without classes,
     * class days), in date order, India time. A day it doesn't list follows the usual timetable
     * (Sundays are off). Empty when the app has no calendar saved.
     *
     * @param fromDaysFromToday The first day: 0 for today, negative for past days, from -180 to 180.
     * @param days How many days, from 1 to 120.
     */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getAcademicCalendar(fromDaysFromToday: Int, days: Int): List<AssistantCalendarDay> {
        if (fromDaysFromToday !in -180..180) throw AppFunctionInvalidArgumentException("fromDaysFromToday must be from -180 to 180")
        if (days !in 1..120) throw AppFunctionInvalidArgumentException("days must be from 1 to 120")
        return read("calendar") { c, f ->
            val from = istDay(System.currentTimeMillis()) + fromDaysFromToday
            c.cachedCalendar(f)?.months.orEmpty().flatMap { it.days }.mapNotNull { d ->
                d.mark?.takeIf { d.day in from until from + days }?.let { AssistantCalendarDay(d.date, it.name.lowercase(), d.title) }
            }
        }
    }

    /**
     * What the student's hostel mess serves on a day, meal by meal (breakfast, lunch, snacks,
     * dinner) with serving times in India time; answers "What's for dinner?" or "What's for lunch
     * tomorrow?". From the monthly mess menu spreadsheet the student opened in the app's Docs. Null
     * when no menu is set up there or it doesn't cover that day.
     *
     * @param daysFromToday 0 for today, 1 for tomorrow, up to 7.
     */
    @AppFunctionDeclaration(isDescribedByKDoc = true)
    suspend fun getMessMenu(daysFromToday: Int): AssistantMessMenu? {
        if (daysFromToday !in 0..7) throw AppFunctionInvalidArgumentException("daysFromToday must be from 0 to 7")
        return read("mess") { _, _ ->
            val doc = Docs.messMenu() ?: return@read null
            val day = istDay(System.currentTimeMillis()) + daysFromToday
            val month = formatDay(day, "M").toInt()
            val menu = parseMessMenu(Docs.withFile(doc, ::readSpreadsheet)) { m -> messMenuYear(m, doc.name, formatDay(day, "yyyy").toInt(), month) }
            // The sheet picked in Docs (MessMenuView's key).
            val chosen = applicationContext.getSharedPreferences("settings", MODE_PRIVATE).getString("mess_menu_sheet", null)
            val sheet = menu.sheets.firstOrNull { it.name == chosen } ?: menu.sheets.first()
            val dom = formatDay(day, "d").toInt()
            if ((menu.month != null && menu.month != month) || !sheet.days.containsKey(dom)) return@read null
            AssistantMessMenu(
                formatDay(day, "yyyy-MM-dd"), sheet.name,
                Meal.entries.map { m ->
                    val w = MessTimings.windows.getValue(m)
                    AssistantMeal(m.label, hhmm(w.start), hhmm(w.end), sheet.itemsFor(dom, m).map { AssistantDish(it.name, it.nonVeg) })
                },
            )
        }
    }

    /** [block] on the app's cache, off the main thread; refused while the setting is off. */
    private suspend fun <T> read(name: String, block: suspend (Core, String) -> T): T = flow("assistant.$name") { f ->
        Settings.load(applicationContext)
        if (!Settings.assistant) {
            Log.info("assistant", "refused: off", f)
            throw AppFunctionPermissionRequiredException("The student has not turned on Assistant in vitapmate's settings")
        }
        AppCore.start(applicationContext)
        withContext(Dispatchers.IO) { block(AppCore.await(), f) }.also { Log.info("assistant", name, f) }
    }

    private fun info(s: ClassSlot, day: Int) = AssistantClass(
        s.name, s.code, formatDay(day, "yyyy-MM-dd"), hhmm(s.start), hhmm(s.end),
        listOf(s.room, s.block).filter { it.isNotEmpty() }.joinToString(" "), s.faculty, s.kind == ClassKind.LAB,
    )

    private fun hhmm(minutes: Int) = "%02d:%02d".format(minutes / 60, minutes % 60)
}

/** Settings → Assistant (Android 16+ only): lets Android's assistant read the saved classes, attendance, exams, marks, grades, outings and mess menu; off by default. */
@Composable
fun AssistantSetting() {
    if (android.os.Build.VERSION.SDK_INT < 36) return
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    val set = { on: Boolean ->
        Settings.assistant = on
        haptics.toggle(on)
        scope.logAction("settings.assistant", "ui.settings", "assistant ${if (on) "on" else "off"}")
    }
    ListGroup("Assistant", footer = "Read only, from what the app has saved. Works with assistants that support Android's app functions.") {
        GroupItem(
            "Answer from my timetable",
            "settings.assistant",
            Lucide.Sparkles,
            "Gemini can ask for your classes, attendance, exams, marks, grades, outings and mess menu",
            onClick = { set(!Settings.assistant) },
            trailing = { ItemSwitch(Settings.assistant, { set(it) }) },
            on = Settings.assistant,
        )
    }
}
