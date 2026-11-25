package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import vitapmate.core.Semester
import vitapmate.core.Semesters

/** The semester list through its cache reads and refreshes. */
class SemesterStateTest {
    private val offline = Problem(ProblemKind.Offline, "Offline", "Connect and try again.", "f1", retryable = true)
    private val list = Semesters(items = listOf(Semester("AP1", "Fall 2026", "Fall", "2026", "")), selected = "AP1", updatedAt = 10)
    private val newer = list.copy(updatedAt = 20)

    private fun SemesterState.after(vararg events: SemesterEvent) = events.fold(this) { s, e -> s.next(e) }

    @Test
    fun aFirstFetchLoadsThenShowsTheListOrWhyNot() {
        assertEquals(SemesterState.Loading, SemesterState.None.next(SemesterEvent.RefreshStarted))
        assertEquals(SemesterState.Ready(list), SemesterState.None.after(SemesterEvent.RefreshStarted, SemesterEvent.RefreshSucceeded(list)))
        assertEquals(SemesterState.Failed(offline), SemesterState.None.after(SemesterEvent.RefreshStarted, SemesterEvent.RefreshFailed(offline)))
        // Stopped without an answer: nothing to show, and not loading any more.
        assertEquals(SemesterState.None, SemesterState.None.after(SemesterEvent.RefreshStarted, SemesterEvent.RefreshCancelled))
    }

    @Test
    fun aCachedListRefreshesInPlaceAndKeepsAFailure() {
        val ready = SemesterState.None.next(SemesterEvent.CacheRead(list))
        assertEquals(SemesterState.Ready(list), ready)
        assertEquals(SemesterState.Ready(list, PageActivity.Refreshing()), ready.next(SemesterEvent.RefreshStarted))
        val stale = ready.after(SemesterEvent.RefreshStarted, SemesterEvent.RefreshFailed(offline))
        assertEquals(SemesterState.Ready(list, PageActivity.Stale(offline)), stale)
        // A cache read (a page fetch cached a newer list) keeps the problem; a success clears it.
        assertEquals(SemesterState.Ready(newer, PageActivity.Stale(offline)), stale.next(SemesterEvent.CacheRead(newer)))
        assertEquals(SemesterState.Ready(newer), stale.after(SemesterEvent.RefreshStarted, SemesterEvent.RefreshSucceeded(newer)))
    }

    @Test
    fun theCacheLandingMidFetchShowsTheFetchStillRunning() {
        assertEquals(SemesterState.Ready(list, PageActivity.Refreshing()), SemesterState.Loading.next(SemesterEvent.CacheRead(list)))
        assertEquals(SemesterState.Ready(list, PageActivity.Stale(offline)), SemesterState.Failed(offline).next(SemesterEvent.CacheRead(list)))
    }

    @Test
    fun theListGoingAwayLeavesNoneOrTheRunningFetch() {
        assertEquals(SemesterState.None, SemesterState.Ready(list).next(SemesterEvent.CacheRead(null)))
        assertEquals(SemesterState.Loading, SemesterState.Ready(list, PageActivity.Refreshing()).next(SemesterEvent.CacheRead(null)))
        assertEquals(SemesterState.Failed(offline), SemesterState.Failed(offline).next(SemesterEvent.CacheRead(null)))
    }
}
