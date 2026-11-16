package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals

class VtopSyncTest {
    @Test
    fun biometricDaysPicksTheNewestIncompleteDaysWithToday() {
        assertEquals(listOf(100, 98, 96), VtopSync.biometricDays(100, setOf(99, 97), 3))
        assertEquals(listOf(100), VtopSync.biometricDays(100, setOf(100) + (87..99), 3))
        assertEquals(listOf(100, 99, 98, 97, 96, 95, 94), VtopSync.biometricDays(100, emptySet(), 7))
        assertEquals(listOf(100, 87), VtopSync.biometricDays(100, (88..99).toSet() + 86, 3))
        assertEquals(emptyList(), VtopSync.biometricDays(100, emptySet(), 0))
    }
}
