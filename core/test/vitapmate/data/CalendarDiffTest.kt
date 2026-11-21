package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals

class CalendarDiffTest {
    private fun ev(key: String, start: Long, title: String = key) = SyncEvent(key, title, start, start + 10)

    @Test
    fun insertsNewUpdatesChangedDeletesGone() {
        val rows = mapOf(1L to ev("a", 100), 2L to ev("b", 100), 3L to ev("gone", 100))
        val d = CalendarDiff.of(rows, listOf(ev("a", 100), ev("b", 100, "B"), ev("new", 200)), from = 0)
        assertEquals(listOf(ev("new", 200)), d.insert)
        assertEquals(mapOf(2L to ev("b", 100, "B")), d.update)
        assertEquals(listOf(3L), d.delete)
    }

    @Test
    fun aSecondSyncWritesNothing() {
        val wanted = listOf(ev("a", 100), ev("b", 200))
        val d = CalendarDiff.of(mapOf(1L to wanted[0], 2L to wanted[1]), wanted, from = 0)
        assertEquals(CalendarDiff(emptyList(), emptyMap(), emptyList()), d)
    }

    @Test
    fun dropsDuplicatesAndRowsWithoutAKey() {
        val d = CalendarDiff.of(mapOf(1L to ev("a", 100), 2L to ev("a", 100), 3L to null), listOf(ev("a", 100)), from = 0)
        assertEquals(emptyList(), d.insert)
        assertEquals(listOf(2L, 3L), d.delete)
    }

    @Test
    fun leavesThePastAlone() {
        // "old" ended before from and is no longer wanted; "past" is wanted but over; "moved" is moved later.
        val rows = mapOf(1L to ev("old", 10), 2L to ev("moved", 10))
        val d = CalendarDiff.of(rows, listOf(ev("past", 20), ev("moved", 500)), from = 100)
        assertEquals(emptyList(), d.insert)
        assertEquals(mapOf(2L to ev("moved", 500)), d.update)
        assertEquals(emptyList(), d.delete)
    }
}
