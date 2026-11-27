package vitapmate.features.attendance

import java.time.LocalDate
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import vitapmate.core.AttendanceCourse
import vitapmate.core.ClassKind

internal fun course(
    attended: Int,
    total: Int,
    counted: Int = total,
    kind: ClassKind = ClassKind.THEORY,
    percent: Double = 0.0,
    safe: Boolean = true,
    canSkip: Int = 0,
    advice: String = "",
    debarred: Boolean = false,
    reportedPercent: Double? = if (total == 0) null else percent,
    rawDebarStatus: String = "",
): AttendanceCourse {
    val lab = kind == ClassKind.LAB
    val sessionsAttended = if (lab) attended / 2 else attended
    val sessionsHeld = if (lab) (counted + 1) / 2 else counted
    val sessionsUnposted = if (lab) ((total - counted).coerceAtLeast(0) + 1) / 2 else (total - counted).coerceAtLeast(0)
    return AttendanceCourse(
        "CSE1001.theory", "AP2026", "ETH", "CSE1001", "Course", kind, "PC", "Faculty",
        attended, total, counted, sessionsAttended, sessionsHeld, sessionsUnposted,
        percent, reportedPercent, safe, canSkip, 0, advice, debarred, null, null, rawDebarStatus,
    )
}

class CalculatorTest {
    @Test
    fun percentTextCutsToOneDecimalAndNeverRoundsUpTo75() {
        assertEquals("0", percentText(0.0))
        assertEquals("75", percentText(75.0))
        assertEquals("100", percentText(100.0))
        assertEquals("74.9", percentText(74.96))
        assertEquals("74.9", percentText(74.99))
        assertEquals("74.5", percentText(74.5))
        assertEquals("33.3", percentText(100.0 / 3))
        assertEquals("66.6", percentText(200.0 / 3))
        assertEquals("80", percentText(80.04))
        // Float error just under a whole tenth still reads as it.
        assertEquals("30", percentText(29.999999999999996))
        assertEquals("70", percentText(0.7 * 100))
    }

    @Test
    fun standingOrdersDebarredFirst() {
        assertEquals(Standing.Debarred, course(0, 0, debarred = true).standing)
        assertEquals(Standing.NoClasses, course(0, 0).standing)
        assertEquals(Standing.Below, course(2, 4, safe = false).standing)
        assertEquals(Standing.Edge, course(3, 4, canSkip = 0).standing)
        assertEquals(Standing.Safe, course(9, 10, canSkip = 2).standing)
    }

    @Test
    fun percentLabelAndAdvice() {
        assertEquals("–", course(0, 0).percentLabel)
        assertEquals("75%", course(149, 200, percent = 74.5).percentLabel)
        assertEquals("74%", course(148, 200, percent = 74.4).percentLabel)
        assertNull(course(0, 0, advice = "No classes yet").advice(showSkip = true))
        assertEquals("Attend 2", course(2, 4, safe = false, advice = "Attend 2").advice(showSkip = false))
        assertNull(course(9, 10, advice = "Can skip 2").advice(showSkip = false))
        assertEquals("Can skip 2", course(9, 10, advice = "Can skip 2").advice(showSkip = true))
        assertEquals("–", course(2, 4, percent = 50.0, reportedPercent = null).reportedPercentLabel)
        assertEquals("75%", course(3, 4, percent = 75.0, reportedPercent = 75.0).reportedPercentLabel)
        assertNull(course(2, 4, rawDebarStatus = "  ").verbatimDebar)
        assertEquals("Permitted", course(2, 4, rawDebarStatus = "Permitted").verbatimDebar)
    }

    @Test
    fun labsSuffix() {
        assertEquals("", course(1, 1).labs(2))
        val lab = course(2, 2, kind = ClassKind.LAB)
        assertEquals(" lab", lab.labs(1))
        assertEquals(" labs", lab.labs(0))
        assertEquals(" labs", lab.labs(3))
    }

    @Test
    fun monthBoundaries() {
        fun day(y: Int, m: Int, d: Int) = LocalDate.of(y, m, d).toEpochDay().toInt()
        assertEquals(0, firstOfMonth(0))
        assertEquals(day(2024, 2, 1), firstOfMonth(day(2024, 2, 29)))
        assertEquals(day(2026, 12, 1), firstOfMonth(day(2026, 12, 31)))
        assertEquals(29, daysInMonth(day(2024, 2, 1)))
        assertEquals(28, daysInMonth(day(2026, 2, 1)))
        assertEquals(31, daysInMonth(day(2026, 12, 1)))
        assertEquals(30, daysInMonth(day(2026, 11, 1)))
    }
}
