package vitapmate

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Utensils
import vitapmate.data.Meal
import vitapmate.data.MessTimings
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.features.docs.MessMenu
import vitapmate.features.docs.messFeature
import vitapmate.features.exams.examsFeature
import vitapmate.features.timetable.istDay
import vitapmate.features.timetable.istMinute
import vitapmate.observability.Log
import vitapmate.ui.Feature

/**
 * One tab: Exams from [EXAM_LEAD] minutes before a paper of today's (in the saved schedule) until
 * [EXAM_TAIL] minutes after it starts, meal or not; else at meal times (from [MEAL_LEAD] minutes before a mess window until it
 * ends) that meal only, named for it; else the whole mess menu. Its label and icon follow.
 * [refresh] reads today's papers from the cache (the shell calls it on start and each time the app
 * comes back); [tick] moves the clock on (each minute while shown).
 */
object MessOrExams {
    const val ID = "mess_or_exams"
    private const val MEAL_LEAD = 30
    private const val EXAM_LEAD = 60
    private const val EXAM_TAIL = 20

    /** Today's papers as minutes since midnight, from [EXAM_LEAD] before the start to [EXAM_TAIL] after it. */
    private var exams by mutableStateOf(emptyList<IntRange>())
    private var minute by mutableIntStateOf(istMinute(System.currentTimeMillis()))

    private val meal: Meal?
        get() = MessTimings.windows.entries.firstOrNull { (_, w) -> minute in w.start - MEAL_LEAD until w.end }?.key

    val feature: Feature
        get() {
            val meal = meal
            return when {
                exams.any { minute in it } -> Feature(ID, examsFeature.label, examsFeature.icon, examsFeature.actions) { examsFeature.content() }
                meal != null -> Feature(ID, meal.label, Lucide.Utensils) { MessMenu(mealOnly = true) }
                else -> Feature(ID, messFeature.label, messFeature.icon) { MessMenu() }
            }
        }

    fun tick() {
        minute = istMinute(System.currentTimeMillis())
    }

    suspend fun refresh() {
        tick()
        val today = istDay(System.currentTimeMillis())
        // A paper without a time (only VTOP's FN/AN session) counts for the whole day.
        val papers = orNull {
            coreFlow("tab.mess_or_exams") { c, f ->
                c.cachedExams(f)?.groups.orEmpty().flatMap { g -> g.papers.filter { it.day == today } }
                    .map { p -> p.start?.let { (it - EXAM_LEAD)..<(it + EXAM_TAIL) } ?: 0..<1440 }
            }
        }.orEmpty()
        if (papers != exams) Log.info("ui.shell", "mess or exams: ${papers.size} papers today")
        exams = papers
    }
}
