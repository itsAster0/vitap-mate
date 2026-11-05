package vitapmate.features.grades

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import vitapmate.core.PlanCourse

/** A planned course: credits and the grade hoped for; code and name when it came from the timetable. */
internal data class PlanRow(val id: Int, val credits: Double = 4.0, val grade: String = "A", val code: String = "", val name: String = "")

/** The GPA planner's inputs (the Flutter app's GPA Planner); kept while the grades screen is open. */
@Stable
internal class Planner {
    val rows = mutableStateListOf(PlanRow(1))
    private var nextId by mutableIntStateOf(2)
    var cgpa by mutableStateOf("")
    var completed by mutableStateOf("")
    /** None: the next quarter point above the current CGPA. */
    var target by mutableStateOf<Double?>(null)
    var editBase by mutableStateOf(false)
    /** This semester's courses from the saved timetable. */
    var semester by mutableStateOf<List<PlanCourse>>(emptyList())
    var prefilled = false
    var autoLoaded = false

    /** Still the one blank row it starts with. */
    val untouched get() = rows.size == 1 && rows[0].code.isEmpty() && rows[0] == PlanRow(rows[0].id)

    fun add() {
        rows += PlanRow(nextId++)
    }

    /** Replaces the rows with this semester's courses. */
    fun loadSemester() {
        if (semester.isEmpty()) return
        rows.clear()
        semester.forEach { rows += PlanRow(nextId++, it.credits, code = it.code, name = it.name) }
    }

    fun update(id: Int, change: (PlanRow) -> PlanRow) {
        val at = rows.indexOfFirst { it.id == id }
        if (at >= 0) rows[at] = change(rows[at])
    }
}
