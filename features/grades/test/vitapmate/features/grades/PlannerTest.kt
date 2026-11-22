package vitapmate.features.grades

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import vitapmate.core.PlanCourse

class PlannerTest {
    @Test
    fun startsWithOneBlankRow() {
        val p = Planner()
        assertTrue(p.untouched)
        assertEquals(listOf(PlanRow(1)), p.rows.toList())
    }

    @Test
    fun addAndUpdateTouchIt() {
        val p = Planner()
        p.update(1) { it.copy(grade = "S") }
        assertFalse(p.untouched)
        assertEquals("S", p.rows[0].grade)
        p.add()
        assertEquals(listOf(1, 2), p.rows.map { it.id })
        // An id not in the rows changes nothing.
        p.update(9) { it.copy(credits = 1.0) }
        assertEquals(listOf(4.0, 4.0), p.rows.map { it.credits })
    }

    @Test
    fun loadSemesterReplacesTheRowsWithFreshIds() {
        val p = Planner()
        p.loadSemester()
        assertEquals(listOf(PlanRow(1)), p.rows.toList())
        p.semester = listOf(PlanCourse("CSE1001", "Problem Solving", 3.0), PlanCourse("CSE1002", "Lab", 1.5))
        p.loadSemester()
        assertEquals(
            listOf(PlanRow(2, 3.0, code = "CSE1001", name = "Problem Solving"), PlanRow(3, 1.5, code = "CSE1002", name = "Lab")),
            p.rows.toList(),
        )
        assertFalse(p.untouched)
        p.add()
        assertEquals(4, p.rows.last().id)
    }

    @Test
    fun numberLabels() {
        assertEquals("9.12", 9.1234.fixed2())
        assertEquals("8.67", 8.666.fixed2())
        assertEquals("0.00", 0.0.fixed2())
        assertEquals("10.00", 10.0.fixed2())
        assertEquals("4", 4.0.creditsLabel())
        assertEquals("0", 0.0.creditsLabel())
        assertEquals("2.5", 2.5.creditsLabel())
        assertEquals("1.3", 1.25.creditsLabel())
    }
}
