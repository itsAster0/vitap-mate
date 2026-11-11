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
import vitapmate.data.AppCore
import vitapmate.data.Settings
import vitapmate.features.timetable.istDay
import vitapmate.features.timetable.istMinute
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
 * App functions for Android's assistant (Android 16+, AppFunctions): the timetable and attendance
 * the app has saved, read only and never from the network. Off until the student turns on
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

    /** [block] on the app's cache, off the main thread; refused while the setting is off. */
    private suspend fun <T> read(name: String, block: (Core, String) -> T): T = flow("assistant.$name") { f ->
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

/** Settings → Assistant (Android 16+ only): lets Android's assistant read the saved timetable and attendance; off by default. */
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
            "Gemini can ask for your classes and attendance",
            onClick = { set(!Settings.assistant) },
            trailing = { ItemSwitch(Settings.assistant, { set(it) }) },
            on = Settings.assistant,
        )
    }
}
