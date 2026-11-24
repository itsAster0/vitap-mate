package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class RefreshTriggerTest {
    @Test
    fun nothingShownAlwaysFetchesOnOpen() {
        assertEquals("open", refreshTrigger(PageWake.Open, ready = false, running = true, autoRefresh = false))
    }

    @Test
    fun aFetchStillRunningIsRejoinedEvenWithAutoRefreshOff() {
        assertEquals("rejoin", refreshTrigger(PageWake.Open, running = true, autoRefresh = false, stale = true))
    }

    @Test
    fun autoRefreshOffLeavesTheCacheOnOpen() {
        assertNull(refreshTrigger(PageWake.Open, autoRefresh = false, fetched = false, stale = true))
    }

    @Test
    fun aSemesterNotFetchedYetFetchesOnOpen() {
        assertEquals("open", refreshTrigger(PageWake.Open, fetched = false, stale = true))
    }

    @Test
    fun oldDataFetchesOnOpenAndFreshDataDoesNot() {
        assertEquals("stale", refreshTrigger(PageWake.Open, stale = true))
        assertNull(refreshTrigger(PageWake.Open))
    }

    @Test
    fun anAnsweredOtpAlwaysFetches() {
        assertEquals("otp", refreshTrigger(PageWake.Otp, autoRefresh = false))
    }

    @Test
    fun comingBackFetchesOnlyOldDataWithAutoRefreshOn() {
        assertEquals("resume", refreshTrigger(PageWake.Resume, stale = true))
        assertNull(refreshTrigger(PageWake.Resume, stale = false))
        assertNull(refreshTrigger(PageWake.Resume, autoRefresh = false, stale = true))
    }

    @Test
    fun consecutiveOtpPromptsWithSameMessageGetDistinctChallengeIds() {
        val before = OtpPrompt.challengeId
        OtpPrompt.ask("Additional verification is required.")
        val first = OtpPrompt.challengeId
        assertEquals(before + 1, first)
        // A second concurrent fetch hitting the same open prompt must not reset the challenge id.
        OtpPrompt.ask("Additional verification is required.")
        assertEquals(first, OtpPrompt.challengeId)
        OtpPrompt.done()
        // A fresh prompt later with the exact same default message gets a new challenge id.
        OtpPrompt.ask("Additional verification is required.")
        assertEquals(first + 1, OtpPrompt.challengeId)
        OtpPrompt.done()
    }
}
