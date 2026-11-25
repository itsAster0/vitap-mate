package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals

/** Every state a ready page's refresh can be in, from each way [rememberPage] gets there. */
class PageActivityTest {
    private val problem = Problem(ProblemKind.Offline, "Offline", "Connect and try again.", "f1", retryable = true)

    @Test
    fun onlyARunningFetchWaitsAndSettlesWithoutAnAnswer() {
        assertEquals(PageActivity.Refreshing("Waiting"), PageActivity.Refreshing().waiting("Waiting"))
        assertEquals(PageActivity.Refreshing(), PageActivity.Refreshing("Waiting").waiting(null))
        assertEquals(PageActivity.Idle, PageActivity.Idle.waiting("Waiting"))
        assertEquals(PageActivity.Stale(problem), PageActivity.Stale(problem).waiting("Waiting"))
        assertEquals(PageActivity.Idle, PageActivity.Refreshing("Waiting").settled())
        assertEquals(PageActivity.Stale(problem), PageActivity.Stale(problem).settled())
        assertEquals(PageActivity.Idle, PageActivity.Idle.settled())
    }
}
