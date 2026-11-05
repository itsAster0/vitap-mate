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
    }
}
