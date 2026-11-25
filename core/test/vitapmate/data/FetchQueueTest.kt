package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** [FetchQueue]'s decisions, event by event; times are the test's. */
class FetchQueueTest {
    private val config = QueueConfig(fetchLanes = 1, queueDropMs = 10_000, slowFetchMs = 5_000)
    private var queue = FetchQueue()

    private fun ticket(key: String, at: Long = 0, user: Boolean = false, hasCache: Boolean = true, background: Boolean = false) =
        Ticket(queue.nextId, key, "f.$key", user, hasCache, background, at)

    private fun on(event: QueueEvent, now: Long = 0): List<QueueEffect> = queue.next(event, now, config).also { queue = it.queue }.effects

    private fun start(key: String, at: Long = 0, user: Boolean = false, hasCache: Boolean = true): Pair<Long, List<QueueEffect>> {
        val t = ticket(key, at, user, hasCache)
        return t.id to on(QueueEvent.Start(t), at)
    }

    private fun List<QueueEffect>.logs() = filterIsInstance<QueueEffect.Log>().map { "${it.msg} ${it.fields["page"]}" }

    @Test
    fun aFreeLaneStartsTheRunAtOnceAndTheNextWaitsWithAQueuedLine() {
        val (a, first) = start("attendance//AP1")
        assertEquals(listOf<QueueEffect>(QueueEffect.Go(a)), first)
        val (b, second) = start("marks//AP1")
        assertTrue(second.none { it is QueueEffect.Go })
        assertEquals(listOf("queued marks"), second.logs())
        assertEquals(listOf("attendance"), queue.runningPages)
        assertEquals(b, queue.active("marks//AP1")?.id)
        // The lane frees: marks starts, and says it waited.
        val done = on(QueueEvent.Finished(a, "attendance//AP1", "f", succeeded = true), now = 300)
        assertTrue(QueueEffect.Go(b) in done)
        assertEquals(listOf("started marks"), done.logs())
        assertEquals(listOf("marks"), queue.runningPages)
    }

    @Test
    fun successIsRememberedAcrossTheNextRunAndAFailureKeepsTheOldOne() {
        val (a, _) = start("timetable//AP1")
        on(QueueEvent.Finished(a, "timetable//AP1", "f", succeeded = true), now = 1_000)
        assertTrue(queue.slots["timetable//AP1"] is Slot.Cooldown)
        assertTrue(queue.succeededWithin("timetable//AP1", 60_000, now = 2_000))
        // Fetched again and failed: the success before it still counts.
        val (b, _) = start("timetable//AP1", at = 2_000)
        assertTrue(queue.succeededWithin("timetable//AP1", 60_000, now = 2_000))
        on(QueueEvent.Finished(b, "timetable//AP1", "f", succeeded = false), now = 3_000)
        assertEquals(Slot.Cooldown(1_000), queue.slots["timetable//AP1"])
        // A failure with no success before leaves nothing.
        val (c, _) = start("marks//AP1")
        on(QueueEvent.Finished(c, "marks//AP1", "f", succeeded = false))
        assertNull(queue.slots["marks//AP1"])
        assertFalse(queue.succeededWithin("marks//AP1", 60_000, now = 0))
    }

    @Test
    fun aLateEndOfAnOlderRunLeavesTheNewOneAlone() {
        val (a, _) = start("marks//AP1")
        on(QueueEvent.CancelAll)
        val (b, _) = start("marks//AP1")
        on(QueueEvent.Finished(a, "marks//AP1", "f", succeeded = false))
        assertEquals(b, queue.active("marks//AP1")?.id)
    }

    @Test
    fun leavingAPageDropsItsStaleAutomaticRunAtOnce() {
        start("profile//AP1")
        on(QueueEvent.Shown("marks", on = true))
        val (marks, _) = start("marks//AP1")
        // Shown: never dropped, however long it waits.
        assertTrue(on(QueueEvent.Fetched("f", 100, udp = false), now = 20_000).none { it is QueueEffect.Drop })
        // The student leaves marks long after it queued: dropped now, not when a lane frees.
        val left = on(QueueEvent.Shown("marks", on = false), now = 20_000)
        assertTrue(QueueEffect.Drop(marks) in left)
        assertEquals(listOf("dropped marks"), left.logs())
        assertNull(queue.active("marks//AP1"))
    }

    @Test
    fun askingForAWaitingRunMovesItAheadAndKeepsItFromBeingDropped() {
        val (blocker, _) = start("profile//AP1")
        val (exams, _) = start("exams//AP1")
        val (marks, _) = start("marks//AP1")
        on(QueueEvent.Ask("marks//AP1"))
        assertTrue(queue.active("marks//AP1")!!.user)
        // Past the drop time: exams goes, marks (asked) stays and gets the lane first.
        val freed = on(QueueEvent.Finished(blocker, "profile//AP1", "f", succeeded = true), now = 20_000)
        assertTrue(QueueEffect.Drop(exams) in freed)
        assertTrue(QueueEffect.Go(marks) in freed)
    }

    @Test
    fun slowFetchesNarrowToOneLaneAndSayWhenTheCountChanges() {
        val two = config.copy(fetchLanes = 2)
        fun at(event: QueueEvent) = queue.next(event, 0, two).also { queue = it.queue }.effects
        at(QueueEvent.Fetched("f", 100, udp = false))
        assertEquals(2, queue.lanes)
        at(QueueEvent.Fetched("f", 9_000, udp = false))
        val slow = at(QueueEvent.Fetched("f", 9_000, udp = false))
        assertEquals(1, queue.lanes)
        assertEquals(listOf("lanes null"), slow.logs())
        assertTrue(queue.slow(two))
        // Over UDP: one lane however fast.
        repeat(3) { at(QueueEvent.Fetched("f", 100, udp = false)) }
        assertFalse(queue.slow(two))
        at(QueueEvent.Fetched("f", 100, udp = true))
        assertTrue(queue.slow(two))
    }

    @Test
    fun signOutCancelsEveryRunAndForgetsSuccesses() {
        val (a, _) = start("profile//AP1")
        on(QueueEvent.Finished(a, "profile//AP1", "f", succeeded = true))
        val (b, _) = start("marks//AP1")
        val (c, _) = start("exams//AP1")
        assertEquals(listOf<QueueEffect>(QueueEffect.Cancel(b), QueueEffect.Cancel(c)), on(QueueEvent.CancelAll))
        assertTrue(queue.slots.isEmpty())
        assertTrue(queue.runningPages.isEmpty())
        // Ids keep counting, so a late end of a cancelled run can't match a new one.
        assertEquals(c + 1, queue.nextId)
    }
}
