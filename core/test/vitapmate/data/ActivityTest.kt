package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/** [Activity.State]: runs moving from running to recent in one step, from any thread. */
class ActivityTest {
    private fun entry(n: Int) = Activity.Entry("f$n", "Refreshing $n", startedAt = n.toLong())

    @Test
    fun aFinishedRunLeavesRunningAndTopsRecent() {
        val a = entry(1)
        val b = entry(2)
        val started = Activity.State().started(a).started(b)
        assertEquals(listOf(a, b), started.running)
        val done = started.finished(a, a.copy(ms = 5))
        assertEquals(listOf(b), done.running)
        assertEquals(listOf(a.copy(ms = 5)), done.recent)
    }

    @Test
    fun recentKeepsTheNewestOnly() {
        var s = Activity.State()
        repeat(Activity.KEEP + 5) { s = s.started(entry(it)).finished(entry(it), entry(it)) }
        assertEquals(Activity.KEEP, s.recent.size)
        assertEquals(entry(Activity.KEEP + 4), s.recent.first())
        assertTrue(s.running.isEmpty())
    }

    @Test
    fun offlineIsTheLatestRunsProblem() {
        val offline = Problem(ProblemKind.Offline, "Offline", "Connect and try again.", "f1", retryable = true)
        val s = Activity.State().started(entry(1)).finished(entry(1), entry(1).copy(problem = offline))
        assertTrue(s.offline)
        assertTrue(!s.started(entry(2)).finished(entry(2), entry(2)).offline)
    }
}
