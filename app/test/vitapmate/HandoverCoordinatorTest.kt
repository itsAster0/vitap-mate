package vitapmate

import java.util.Collections
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNotEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.runBlocking

class HandoverCoordinatorTest {
    @Test
    fun expiredRequestsAreDroppedBeforeNotifyingOrAnswering() {
        val coordinator = HandoverCoordinator(clock = { 10_000L })

        val bgPast = coordinator.onRequest("req-past", expiresAtMs = 9_999L, foreground = false)
        assertIs<HandoverAction.Expired>(bgPast)
        assertEquals(HandoverCoordinator.notificationId("req-past"), bgPast.notificationId)
        assertNull(coordinator.stateOf("req-past"))

        val fgBoundary = coordinator.onRequest("req-now", expiresAtMs = 10_000L, foreground = true)
        assertIs<HandoverAction.Expired>(fgBoundary)
        assertNull(coordinator.stateOf("req-now"))
        assertEquals(0, coordinator.size)
    }

    @Test
    fun backgroundNotifiedRequestPromotesOnForegroundWithoutDoubleAnswering() {
        val coordinator = HandoverCoordinator(clock = { 1_000L })

        val first = coordinator.onRequest("req-1", expiresAtMs = 60_000L, foreground = false)
        assertIs<HandoverAction.Notify>(first)
        assertEquals(HandoverState.Notified(60_000L), coordinator.stateOf("req-1"))

        // Duplicate background FCM while already notified must not repost.
        assertEquals(HandoverAction.Ignore, coordinator.onRequest("req-1", expiresAtMs = 60_000L, foreground = false))

        val promoted = coordinator.promoteNotified()
        assertEquals(emptyList(), promoted.expiredNotificationIds)
        assertEquals(1, promoted.toAnswer.size)
        val item = promoted.toAnswer.single()
        assertEquals("req-1", item.requestId)
        assertEquals(first.notificationId, item.notificationId)
        assertEquals(HandoverState.Answering(60_000L, item.attemptId), coordinator.stateOf("req-1"))

        // Re-entering foreground, live follow, or tapping while Answering must all deduplicate.
        assertTrue(coordinator.promoteNotified().toAnswer.isEmpty())
        assertEquals(HandoverAction.Ignore, coordinator.onRequest("req-1", expiresAtMs = 60_000L, foreground = true))
        assertEquals(HandoverAction.Ignore, coordinator.onRequest("req-1", expiresAtMs = null, foreground = true, tapped = true))

        assertTrue(coordinator.complete("req-1", item.attemptId))
        assertEquals(HandoverState.Done(60_000L), coordinator.stateOf("req-1"))

        // Terminal Done deduplicates every entry point.
        assertEquals(HandoverAction.Ignore, coordinator.onRequest("req-1", expiresAtMs = 60_000L, foreground = true))
        assertEquals(HandoverAction.Ignore, coordinator.onRequest("req-1", expiresAtMs = 60_000L, foreground = false))
        assertEquals(HandoverAction.Ignore, coordinator.onRequest("req-1", expiresAtMs = null, foreground = false, tapped = true))
    }

    @Test
    fun notifiedRequestPromotesOnTapAndDeduplicatesFollowAndFcm() {
        val coordinator = HandoverCoordinator(clock = { 1_000L })

        val notify = coordinator.onRequest("req-tap", expiresAtMs = 50_000L, foreground = false)
        assertIs<HandoverAction.Notify>(notify)

        val tap = coordinator.onRequest("req-tap", expiresAtMs = null, foreground = false, tapped = true)
        assertIs<HandoverAction.Answer>(tap)
        assertEquals(notify.notificationId, tap.notificationId)
        assertEquals(50_000L, tap.expiresAtMs)
        assertEquals(HandoverState.Answering(50_000L, tap.attemptId), coordinator.stateOf("req-tap"))

        assertEquals(HandoverAction.Ignore, coordinator.onRequest("req-tap", expiresAtMs = 50_000L, foreground = true))
        assertTrue(coordinator.promoteNotified().toAnswer.isEmpty())
    }

    @Test
    fun fcmWhileForegroundedStartsAnsweringDirectlyAndDeduplicatesLiveFollow() {
        val coordinator = HandoverCoordinator(clock = { 1_000L })

        val fcm = coordinator.onRequest("req-fg", expiresAtMs = 121_000L, foreground = true)
        assertIs<HandoverAction.Answer>(fcm)
        assertEquals(HandoverState.Answering(121_000L, fcm.attemptId), coordinator.stateOf("req-fg"))

        assertEquals(HandoverAction.Ignore, coordinator.onRequest("req-fg", expiresAtMs = 120_000L, foreground = true))
    }

    @Test
    fun notifiedRequestExpiredBeforeForegroundOrTapIsDroppedAndNotificationCleared() {
        var now = 1_000L
        val coordinator = HandoverCoordinator(clock = { now }, pruneGraceMs = 10_000L)

        // Advance to expiry (within grace window): tap with null expiresAt still recognizes expiry.
        val tapNotify = assertIs<HandoverAction.Notify>(coordinator.onRequest("req-exp-tap", expiresAtMs = 5_000L, foreground = false))
        now = 5_000L
        val tapExpired = coordinator.onRequest("req-exp-tap", expiresAtMs = null, foreground = true, tapped = true)
        assertIs<HandoverAction.Expired>(tapExpired)
        assertEquals(tapNotify.notificationId, tapExpired.notificationId)
        assertEquals(HandoverState.Done(5_000L), coordinator.stateOf("req-exp-tap"))

        // Freshly expired Notified entries are reported by promoteNotified so their notifications clear.
        val promoteNotify = assertIs<HandoverAction.Notify>(coordinator.onRequest("req-exp-promote", expiresAtMs = 6_000L, foreground = false))
        now = 6_000L
        val promotion = coordinator.promoteNotified()
        assertTrue(promotion.toAnswer.isEmpty())
        assertEquals(listOf(promoteNotify.notificationId), promotion.expiredNotificationIds)

        // Even past the prune grace window, a tap with null expiresAt on a still-stored Notified entry is dropped.
        val wayPastNotify = assertIs<HandoverAction.Notify>(coordinator.onRequest("req-exp-way-past", expiresAtMs = 7_000L, foreground = false))
        now = 100_000L
        val wayPast = coordinator.onRequest("req-exp-way-past", expiresAtMs = null, foreground = true, tapped = true)
        assertIs<HandoverAction.Expired>(wayPast)
        assertEquals(wayPastNotify.notificationId, wayPast.notificationId)
        assertNull(coordinator.stateOf("req-exp-way-past"))
    }

    @Test
    fun directCompleteAndTimeoutRaceAllowsOnlyOneWinner() {
        val coordinator = HandoverCoordinator(clock = { 1_000L })

        val a1 = assertIs<HandoverAction.Answer>(coordinator.onRequest("req-a", 60_000L, foreground = true))
        val job1 = Job()
        assertTrue(coordinator.attachJob("req-a", a1.attemptId, job1))
        assertTrue(coordinator.complete("req-a", a1.attemptId))
        assertFalse(coordinator.timeout("req-a", a1.attemptId))
        assertFalse(coordinator.complete("req-a", a1.attemptId))
        assertEquals(HandoverState.Done(60_000L), coordinator.stateOf("req-a"))

        val a2 = assertIs<HandoverAction.Answer>(coordinator.onRequest("req-b", 60_000L, foreground = true))
        val job2 = Job()
        assertTrue(coordinator.attachJob("req-b", a2.attemptId, job2))
        // Wrong attemptId is ignored.
        assertFalse(coordinator.timeout("req-b", a2.attemptId + 99))
        assertFalse(job2.isCancelled)

        assertTrue(coordinator.timeout("req-b", a2.attemptId))
        assertTrue(job2.isCancelled)
        assertFalse(coordinator.complete("req-b", a2.attemptId))
        assertFalse(coordinator.timeout("req-b", a2.attemptId))
        assertEquals(HandoverState.Done(60_000L), coordinator.stateOf("req-b"))
    }

    @Test
    fun timeoutCancelsInFlightJobAndSendsExactlyOneTimeoutAnswer() = runBlocking {
        val coordinator = HandoverCoordinator(clock = { 1_000L })
        val action = assertIs<HandoverAction.Answer>(coordinator.onRequest("req-timeout", 60_000L, foreground = true))

        val started = CompletableDeferred<Unit>()
        var cancelled = false
        val sentErrors = Collections.synchronizedList(mutableListOf<String?>())

        val result = coordinator.executeAttempt(
            requestId = "req-timeout",
            attemptId = action.attemptId,
            timeoutMs = 10_000L,
            timeoutError = "timed out",
            dispatcher = Dispatchers.Default,
            waitForWork = {
                started.await()
                false // Simulate timeout firing while answer(null) is still suspended.
            },
        ) { error ->
            if (error == null) {
                started.complete(Unit)
                try {
                    CompletableDeferred<Unit>().await()
                } finally {
                    cancelled = true
                }
            } else {
                sentErrors += error
            }
        }

        assertTrue(result.isSuccess)
        assertTrue(cancelled)
        assertEquals(listOf<String?>("timed out"), sentErrors)
        assertEquals(HandoverState.Done(60_000L), coordinator.stateOf("req-timeout"))
    }

    @Test
    fun completionWinningTimeoutRaceNeverSendsSecondAnswer() = runBlocking {
        val coordinator = HandoverCoordinator(clock = { 1_000L })
        val action = assertIs<HandoverAction.Answer>(coordinator.onRequest("req-race", 60_000L, foreground = true))

        val sentErrors = Collections.synchronizedList(mutableListOf<String?>())

        val result = coordinator.executeAttempt(
            requestId = "req-race",
            attemptId = action.attemptId,
            timeoutMs = 10_000L,
            timeoutError = "timed out",
            dispatcher = Dispatchers.Default,
            waitForWork = { job ->
                // Let the real answer finish, then still report timeout to exercise the boundary race.
                job.join()
                false
            },
        ) { error ->
            sentErrors += error
        }

        assertTrue(result.isSuccess)
        assertEquals(listOf<String?>(null), sentErrors)
        assertEquals(HandoverState.Done(60_000L), coordinator.stateOf("req-race"))
    }

    @Test
    fun failureCompletionWinningTimeoutRaceReturnsFailureWithoutTimeoutAnswer() = runBlocking {
        val coordinator = HandoverCoordinator(clock = { 1_000L })
        val action = assertIs<HandoverAction.Answer>(coordinator.onRequest("req-fail-race", 60_000L, foreground = true))

        val sentErrors = Collections.synchronizedList(mutableListOf<String?>())

        val result = coordinator.executeAttempt(
            requestId = "req-fail-race",
            attemptId = action.attemptId,
            timeoutMs = 10_000L,
            timeoutError = "timed out",
            dispatcher = Dispatchers.Default,
            waitForWork = { job ->
                job.join()
                false
            },
        ) { error ->
            sentErrors += error
            if (error == null) error("consent denied")
        }

        assertTrue(result.isFailure)
        assertEquals("consent denied", result.exceptionOrNull()?.message)
        assertEquals(listOf<String?>(null), sentErrors)
        assertEquals(HandoverState.Done(60_000L), coordinator.stateOf("req-fail-race"))
    }

    @Test
    fun executeAttemptSkipsCallWhenRequestExpiresBeforeRunning() = runBlocking {
        var now = 9_500L
        val coordinator = HandoverCoordinator(clock = { now })
        val action = assertIs<HandoverAction.Answer>(coordinator.onRequest("req-late", 10_000L, foreground = true))

        now = 10_000L
        var called = false
        val result = coordinator.executeAttempt(
            requestId = "req-late",
            attemptId = action.attemptId,
            timeoutMs = 10_000L,
            timeoutError = "timed out",
            dispatcher = Dispatchers.Default,
        ) {
            called = true
        }

        assertTrue(result.isSuccess)
        assertFalse(called)
        assertEquals(HandoverState.Done(10_000L), coordinator.stateOf("req-late"))
    }

    @Test
    fun expiredEntriesArePrunedToKeepMapBounded() {
        var now = 1_000L
        val coordinator = HandoverCoordinator(clock = { now }, pruneGraceMs = 5_000L)

        for (i in 1..20) {
            val action = assertIs<HandoverAction.Answer>(coordinator.onRequest("done-$i", 10_000L, foreground = true))
            assertTrue(coordinator.complete("done-$i", action.attemptId))
        }
        val active = assertIs<HandoverAction.Answer>(coordinator.onRequest("active", 10_000L, foreground = true))
        val notified = assertIs<HandoverAction.Notify>(coordinator.onRequest("notified", 10_000L, foreground = false))
        assertEquals(22, coordinator.size)

        // Still inside grace window: Done entries remain to block late duplicates; Notified flips to Done.
        now = 12_000L
        val expiredIds = coordinator.prune()
        assertEquals(listOf(notified.notificationId), expiredIds)
        assertEquals(22, coordinator.size)
        assertEquals(HandoverState.Done(10_000L), coordinator.stateOf("notified"))

        // Past expiresAt + pruneGraceMs: all Done entries are pruned; active Answering is untouched.
        now = 15_000L
        assertEquals(emptyList(), coordinator.prune())
        assertEquals(1, coordinator.size)
        assertEquals(HandoverState.Answering(10_000L, active.attemptId), coordinator.stateOf("active"))

        // Completing an Answering entry past grace removes it immediately.
        assertTrue(coordinator.complete("active", active.attemptId))
        assertEquals(0, coordinator.size)
    }

    @Test
    fun uniqueNotificationIdsPerRequestId() {
        val id1 = HandoverCoordinator.notificationId("req-alpha")
        val id2 = HandoverCoordinator.notificationId("req-beta")
        assertEquals(id1, HandoverCoordinator.notificationId("req-alpha"))
        assertNotEquals(id1, id2)
        assertTrue(id1 > 0)
        assertTrue(id2 > 0)
    }

    @Test
    fun parsePushExpiresAtMsHandlesSecondsMillisAtAndMissing() {
        assertEquals(1_700_000_120_000L, HandoverCoordinator.parsePushExpiresAtMs(mapOf("expiresAt" to "1700000120000")))
        assertEquals(1_700_000_120_000L, HandoverCoordinator.parsePushExpiresAtMs(mapOf("expiresAt" to "1700000120")))
        assertEquals(
            1_700_000_000_000L + HandoverCoordinator.DEFAULT_REQUEST_TTL_MS,
            HandoverCoordinator.parsePushExpiresAtMs(mapOf("at" to "1700000000")),
        )
        assertNull(HandoverCoordinator.parsePushExpiresAtMs(mapOf("kind" to "vtop_cookie_request")))
        assertNull(HandoverCoordinator.parsePushExpiresAtMs(mapOf("at" to "not-a-number")))
    }
}
