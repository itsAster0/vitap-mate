package vitapmate.features.attendance

import java.time.LocalDate
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
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
) = AttendanceCourse(
    "CSE1001.theory", "AP2026", "ETH", "CSE1001", "Course", kind, "PC", "Faculty",
    attended, total, counted, percent, safe, canSkip, 0, advice, debarred, null, null,
)

class CalculatorTest {
    @Test
    fun projectedWithNoClassesIsZeroAndSafe() {
        assertEquals(0.0 to true, course(0, 0).projected(0, 0))
        assertEquals(0.0 to true, course(0, 0).projected(0, 0, od = 0))
        // The first class missed: 0 of 1.
        assertEquals(0.0 to false, course(0, 0).projected(0, 1))
        assertEquals(100.0 to true, course(0, 0).projected(1, 0))
    }

    @Test
    fun exactlySeventyFivePercentIsSafe() {
        assertEquals(75.0 to true, course(3, 4).projected(0, 0))
        assertEquals(75.0 to true, course(30, 40).projected(0, 0))
        val (p, safe) = course(149, 200).projected(0, 0)
        assertEquals(74.5, p)
        assertFalse(safe)
    }

    @Test
    fun projectedAddsAttendedSkippedAndOnDuty() {
        // 2 of 4, then attend 4 more: 6 of 8.
        assertEquals(75.0 to true, course(2, 4).projected(4, 0))
        assertFalse(course(2, 4).projected(3, 0).second)
        // 9 of 10: two misses keep 75%, a third doesn't.
        assertEquals(75.0 to true, course(9, 10).projected(0, 2))
        assertFalse(course(9, 10).projected(0, 3).second)
        // OD turns an absence present without adding a class.
        assertEquals(75.0 to true, course(2, 4).projected(0, 0, od = 1))
        assertEquals(50.0 to false, course(2, 4).projected(1, 1, od = 0))
    }

    @Test
    fun projectedUsesCountedNotTotal() {
        // 2 classes held but not posted yet are left out.
        assertEquals(75.0 to true, course(3, 6, counted = 4).projected(0, 0))
    }

    @Test
    fun labsCountInTwoPeriodSessions() {
        val lab = course(6, 8, kind = ClassKind.LAB)
        assertEquals(75.0 to true, lab.projected(0, 0))
        assertEquals(80.0 to true, lab.projected(1, 0))
        val (p, safe) = lab.projected(0, 1)
        assertEquals(60.0, p)
        assertFalse(safe)
        assertEquals((8 * 100.0 / 12) to false, lab.projected(1, 1))
    }

    @Test
    fun sessionsHalveLabsAndRoundAnOddTotalUp() {
        assertEquals(5 to 7, course(5, 7).sessions)
        assertEquals(2 to 4, course(5, 7, kind = ClassKind.LAB).sessions)
        assertEquals(3 to 4, course(6, 8, kind = ClassKind.LAB).sessions)
        assertEquals(0 to 0, course(0, 0, kind = ClassKind.LAB).sessions)
        assertEquals(3 to 4, course(6, 10, counted = 8, kind = ClassKind.LAB).sessions)
    }

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
    fun goalSplitWithNothingHeldOrLeft() {
        assertEquals(0 to 0, goalSplit(75, attended = 0, held = 0, left = 0))
        assertEquals(0 to 0, goalSplit(75, attended = 3, held = 4, left = 0))
        // Below the goal with nothing left: can't reach it.
        assertEquals(null to 0, goalSplit(75, attended = 2, held = 4, left = 0))
    }

    @Test
    fun goalSplitAtExactlySeventyFive() {
        // 45 of 60 is exactly 75%.
        assertEquals(15 to 5, goalSplit(75, attended = 30, held = 40, left = 20))
        // 7.5 of 10 rounds up to 8.
        assertEquals(8 to 2, goalSplit(75, attended = 0, held = 0, left = 10))
        // Already over: every class left can be missed.
        assertEquals(0 to 10, goalSplit(75, attended = 40, held = 40, left = 10))
    }

    @Test
    fun goalSplitOutOfReach() {
        assertEquals(null to 0, goalSplit(90, attended = 30, held = 40, left = 10))
        assertEquals(10 to 0, goalSplit(80, attended = 30, held = 40, left = 10))
    }

    @Test
    fun goalSplitMatchesTheFewestClassesThatReachTheGoal() {
        for (goal in listOf(75, 80, 85, 90)) {
            for (held in 0..14) for (attended in 0..held) for (left in 0..14) {
                val total = held + left
                val fewest = (0..left).firstOrNull { m -> (attended + m) * 100 >= goal * total }
                val (must, canMiss) = goalSplit(goal, attended, held, left)
                val at = "goal $goal, $attended of $held, $left left"
                assertEquals(fewest, must, at)
                assertEquals(if (fewest == null) 0 else left - fewest, canMiss, at)
                if (goal == 75 && must != null) {
                    // The split played out in What if lands at 75% or above.
                    assertTrue(course(attended, held).projected(must, canMiss).second, at)
                }
            }
        }
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
