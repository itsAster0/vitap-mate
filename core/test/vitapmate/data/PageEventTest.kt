package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue

/** A page's state through [PageHandle]'s events, in the orders they can come. */
class PageEventTest {
    private val offline = Problem(ProblemKind.Offline, "Offline", "Connect and try again.", "f1", retryable = true)
    private val loading: PageState<String> = PageState.Loading()

    private fun PageState<String>.after(vararg events: PageEvent<String>) = events.fold(this) { s, e -> s.next(e) }

    @Test
    fun theCacheShowsFirstThenTheFetchRefreshesIt() {
        val cached = loading.after(PageEvent.CacheLoaded("old", 10))
        assertEquals(PageState.Ready("old", 10), cached)
        val refreshing = cached.after(PageEvent.FetchStarted, PageEvent.WaitingChanged("Waiting for Marks to finish"))
        assertEquals(PageState.Ready("old", 10, PageActivity.Refreshing("Waiting for Marks to finish")), refreshing)
        assertTrue(refreshing.fetching)
        assertEquals(PageState.Ready("new", 20), refreshing.after(PageEvent.WaitingChanged(null), PageEvent.FetchSucceeded("new", 20)))
    }

    @Test
    fun aFailureKeepsTheCacheOrFailsWithNone() {
        assertEquals(PageState.Ready("old", 10, PageActivity.Stale(offline)), loading.after(PageEvent.CacheLoaded("old", 10), PageEvent.FetchStarted, PageEvent.FetchFailed(offline)))
        assertEquals(PageState.Failed(offline), loading.after(PageEvent.FetchStarted, PageEvent.FetchFailed(offline)))
        // A retry runs: the old problem is not shown again while it does.
        assertEquals(PageState.Loading(), PageState.Failed(offline).after(PageEvent.FetchStarted))
    }

    @Test
    fun freshDataIsNeverOverwrittenByTheCache() {
        // A cache read that comes back late finds the fresh copy and leaves it.
        val fresh = loading.after(PageEvent.FetchStarted, PageEvent.FetchSucceeded("new", 20))
        assertEquals(fresh, fresh.after(PageEvent.CacheLoaded("old", 10)))
        // Over a failed fetch, the cache shows with the problem.
        assertEquals(PageState.Ready("old", 10, PageActivity.Stale(offline)), PageState.Failed(offline).after(PageEvent.CacheLoaded("old", 10)))
    }

    @Test
    fun aFetchEndingWithoutAnAnswerNeverLeavesTheSpinner() {
        // Cancelled (sign-out) or dropped with nothing shown: a problem to retry from, not Loading forever.
        val cancelled = loading.after(PageEvent.FetchStarted, PageEvent.FetchCancelled("f2"))
        assertIs<PageState.Failed>(cancelled)
        assertEquals("f2", cancelled.problem.flow)
        assertTrue(cancelled.problem.retryable)
        assertFalse(cancelled.problem.userInitiated)
        assertFalse(cancelled.fetching)
        assertIs<PageState.Failed>(loading.after(PageEvent.FetchStarted, PageEvent.FetchDropped("f3")))
        // With a copy shown it settles back to it.
        val ready = PageState.Ready("old", 10, PageActivity.Refreshing("Waiting"))
        assertEquals(PageState.Ready("old", 10), ready.next(PageEvent.FetchCancelled("f2")))
        assertEquals(PageState.Ready("old", 10), ready.next(PageEvent.FetchDropped("f3")))
        // A problem already shown stays.
        assertEquals(PageState.Ready("old", 10, PageActivity.Stale(offline)), PageState.Ready("old", 10, PageActivity.Stale(offline)).next(PageEvent.FetchCancelled("f2")))
    }

    @Test
    fun onlyAFetchWaitsForALane() {
        assertEquals(PageState.Loading("Waiting"), loading.next(PageEvent.WaitingChanged("Waiting")))
        assertEquals(PageState.Failed(offline), PageState.Failed(offline).next(PageEvent.WaitingChanged("Waiting")))
        assertEquals(PageState.Ready("old", 10), PageState.Ready("old", 10).next(PageEvent.WaitingChanged("Waiting")))
    }
}
