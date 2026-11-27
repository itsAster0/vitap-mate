package vitapmate.features.timetable

import kotlin.test.Test
import kotlin.test.assertEquals
import vitapmate.core.AttendanceCourse
import vitapmate.core.ClassKind

class SkipHintTest {
    private fun course(safe: Boolean, canSkip: Int, mustAttend: Int) = AttendanceCourse(
        "CSE1001.theory", "AP2026", "ETH", "CSE1001", "Course", ClassKind.THEORY, "PC", "Faculty",
        0, 0, 0, 0, 0, 0, 0.0, 0.0, safe, canSkip, mustAttend, "", false, null, null, "",
    )

    @Test
    fun hintFollowsTheStanding() {
        assertEquals("need 2", skipHint(course(safe = false, canSkip = 0, mustAttend = 2)))
        assertEquals("don't skip", skipHint(course(safe = true, canSkip = 0, mustAttend = 0)))
        assertEquals("skip 3", skipHint(course(safe = true, canSkip = 3, mustAttend = 0)))
    }
}
