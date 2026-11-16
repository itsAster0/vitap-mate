package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals

class BiometricFetchTest {
    @Test
    fun windowsCoverTheDaysInFourteenDayRanges() {
        assertEquals(emptyList(), BiometricFetch.windows(emptyList()))
        assertEquals(listOf(100..113), BiometricFetch.windows(listOf(100)))
        assertEquals(listOf(100..113, 114..127), BiometricFetch.windows(listOf(113, 100, 114, 127, 100)))
        assertEquals(listOf(100..113, 200..213), BiometricFetch.windows(listOf(100, 105, 200)))
        assertEquals(9, BiometricFetch.windows((0 until 120).toList()).size)
    }

    @Test
    fun stopNamesTheSemestersUniqueWorkWithItsOwnRequestCode() {
        assertEquals("wrapped.biometric.AP2024253", BiometricFetch.work("AP2024253"))
        assertEquals(BiometricFetch.stopCode("AP2026272"), BiometricFetch.stopCode("AP2026272"))
        assertNotEquals(BiometricFetch.stopCode("AP2026272"), BiometricFetch.stopCode("AP2026275"))
    }

    @Test
    fun progressResetsTheStallsAndTheFifthStallGivesUp() {
        assertEquals(BiometricFetch.Next.Rerun, BiometricFetch.afterStall(fetched = 3, attempt = 4))
        assertEquals(BiometricFetch.Next.Retry, BiometricFetch.afterStall(fetched = 0, attempt = 0))
        assertEquals(BiometricFetch.Next.Retry, BiometricFetch.afterStall(fetched = 0, attempt = 3))
        assertEquals(BiometricFetch.Next.GiveUp, BiometricFetch.afterStall(fetched = 0, attempt = 4))
    }
}
