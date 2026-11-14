package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ForegroundTest {
    @Test
    fun onlyAComebackCountsAsAResume() {
        val start = Foreground.resumes
        Foreground.started() // the first start is not a comeback
        assertEquals(start, Foreground.resumes)
        assertTrue(Foreground.shown)
        Foreground.stopped()
        assertFalse(Foreground.shown)
        Foreground.started()
        assertEquals(start + 1, Foreground.resumes)
        Foreground.stopped()
    }

    @Test
    fun anOldActivityStoppingAfterANewOneStartedLeavesTheAppShown() {
        Foreground.started()
        Foreground.started() // a second MainActivity, e.g. opened from the launcher
        Foreground.stopped() // the first one's onStop comes after the second's onStart
        assertTrue(Foreground.shown)
        Foreground.stopped()
        assertFalse(Foreground.shown)
    }
}
