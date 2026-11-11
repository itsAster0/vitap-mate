package vitapmate.data

import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield

class SingleFlightTest {
    @Test
    fun aCallerLeavingDoesNotCancelTheRunAndTheNextOneJoinsIt() = runBlocking {
        val flights = SingleFlight(CoroutineScope(coroutineContext + SupervisorJob()))
        val gate = CompletableDeferred<Unit>()
        var runs = 0
        val work: suspend () -> String = {
            runs++
            gate.await()
            "fresh"
        }

        // The tab that started the fetch goes away while it runs.
        val first = flights.start("attendance//AP1", "f1", work = work)
        val tab = launch { first.result.await() }
        yield()
        tab.cancel()
        assertFalse(first.joined)

        // Shown again: it joins the same run, named by the flow that runs it.
        val again = flights.start("attendance//AP1", "f2", work = work)
        assertTrue(again.joined)
        assertEquals("f1", again.flow)
        // Another semester or page is its own run.
        val other = flights.start("attendance//AP2", "f3") { "other" }
        assertFalse(other.joined)

        gate.complete(Unit)
        assertEquals("fresh", again.result.await())
        assertEquals("other", other.result.await())
        assertEquals(1, runs)

        // Done: the next start runs again.
        assertFalse(flights.start("attendance//AP1", "f4", work = work).also { it.result.await() }.joined)
        assertEquals(2, runs)
    }

    @Test
    fun aRunThatJustSucceededIsKnownForAWhile() = runBlocking {
        var now = 1_000L
        val flights = SingleFlight(CoroutineScope(coroutineContext + SupervisorJob())) { now }
        val gate = CompletableDeferred<Unit>()
        val run = flights.start("timetable//AP1", "f1") { gate.await() }
        assertTrue(flights.isRunning("timetable//AP1"))
        assertFalse(flights.succeededWithin("timetable//AP1", 60_000))
        gate.complete(Unit)
        run.result.await()
        yield() // the completion handler
        assertFalse(flights.isRunning("timetable//AP1"))
        // The screen comes back: the fetch that finished meanwhile counts, for a while.
        now += 59_000
        assertTrue(flights.succeededWithin("timetable//AP1", 60_000))
        assertFalse(flights.succeededWithin("timetable//AP2", 60_000))
        now += 2_000
        assertFalse(flights.succeededWithin("timetable//AP1", 60_000))
        // A failure doesn't count.
        val failed = flights.start("marks//AP1", "f2") { error("offline") }
        runCatching { failed.result.await() }
        assertFalse(flights.succeededWithin("marks//AP1", 60_000))
        // Sign-out forgets.
        flights.start("timetable//AP1", "f3") { }.result.await()
        flights.cancelAll()
        assertFalse(flights.succeededWithin("timetable//AP1", 60_000))
    }

    @Test
    fun signOutCancelsEveryRun() = runBlocking {
        val flights = SingleFlight(CoroutineScope(coroutineContext + SupervisorJob()))
        val run = flights.start("marks//AP1", "f1") { CompletableDeferred<Unit>().await() }
        flights.cancelAll()
        assertTrue(run.result.isCancelled)
        assertFalse(flights.start("marks//AP1", "f2") { }.joined)
    }

    // Lanes and priorities. The clock is the test's; log lines are collected instead of sent.

    private var now = 1_000L
    private val logged = mutableListOf<String>()

    private fun CoroutineScope.lanes() = SingleFlight(CoroutineScope(coroutineContext + SupervisorJob()), { m, _, x -> logged += "$m ${x["page"]}" }) { now }

    /** A run that holds its lane until [gate] completes; returns its key's page id when done. */
    private fun SingleFlight.hold(key: String, gate: Deferred<Unit>, user: Boolean = false, hasCache: Boolean = true, background: Boolean = false) =
        start(key, "f.$key", user, hasCache, background) { gate.await(); key.substringBefore('/') }

    /** Lets the started runs reach their work and finished ones free their lanes. */
    private suspend fun settle() = repeat(5) { yield() }

    @AfterTest
    fun defaults() = Tunable.entries.forEach { Settings.setTunable(it, null) }

    @Test
    fun atMostTheLaneCountRunsAtOnce() = runBlocking {
        val flights = lanes()
        val gates = List(3) { CompletableDeferred<Unit>() }
        val runs = gates.mapIndexed { i, g -> flights.hold("page$i//AP1", g) }
        settle()
        assertEquals(listOf(true, true, false), runs.map { it.started.isCompleted })
        // Waiting counts as running: a page shown again joins it.
        assertTrue(flights.isRunning("page2//AP1"))
        assertTrue(flights.hold("page2//AP1", gates[2]).joined)
        assertEquals(listOf("page0", "page1"), flights.runningPages.toList())
        gates[0].complete(Unit)
        runs[0].result.await()
        settle()
        assertTrue(runs[2].started.isCompleted)
        assertTrue("queued page2" in logged && "started page2" in logged)
        gates.forEach { it.complete(Unit) }
        runs.forEach { it.result.await() }
    }

    @Test
    fun onScreenBeatsLeftBeatsBackgroundAndUserBeatsAutomatic() = runBlocking {
        Settings.setTunable(Tunable.FetchLanes, 1)
        val flights = lanes()
        flights.shown("attendance", true)
        val gate = CompletableDeferred<Unit>()
        val blocker = flights.hold("profile//AP1", gate)
        val order = mutableListOf<String>()
        fun SingleFlight.queue(key: String, user: Boolean = false, background: Boolean = false) =
            start(key, "f.$key", user, hasCache = true, background = background) { order += key }
        val runs = listOf(
            flights.queue("marks/stale/", background = true),
            flights.queue("timetable//AP1"),
            flights.queue("exams//AP1", user = true),
            flights.queue("attendance//AP1"),
        )
        settle()
        gate.complete(Unit)
        blocker.result.await()
        runs.forEach { it.result.await() }
        assertEquals(listOf("attendance//AP1", "exams//AP1", "timetable//AP1", "marks/stale/"), order)
    }

    @Test
    fun aWaitingRunMovesWithItsPage() = runBlocking {
        Settings.setTunable(Tunable.FetchLanes, 1)
        val flights = lanes()
        val gate = CompletableDeferred<Unit>()
        val blocker = flights.hold("profile//AP1", gate)
        val order = mutableListOf<String>()
        flights.shown("marks", true)
        val marks = flights.start("marks//AP1", "f1", user = false) { order += "marks" }
        val exams = flights.start("exams//AP1", "f2", user = false) { order += "exams" }
        // The student leaves marks for exams before a lane frees: nothing is cancelled, only moved.
        flights.shown("marks", false)
        flights.shown("exams", true)
        assertFalse(marks.result.isCancelled)
        gate.complete(Unit)
        blocker.result.await()
        marks.result.await()
        exams.result.await()
        assertEquals(listOf("exams", "marks"), order)
    }

    @Test
    fun aStaleAutomaticRunOffScreenIsDroppedButNotAUserOrUncachedOne() = runBlocking {
        Settings.setTunable(Tunable.FetchLanes, 1)
        val flights = lanes()
        val gate = CompletableDeferred<Unit>()
        val blocker = flights.hold("profile//AP1", gate)
        val done = CompletableDeferred<Unit>().apply { complete(Unit) }
        val auto = flights.hold("marks//AP1", done)
        val user = flights.hold("exams//AP1", done, user = true)
        val uncached = flights.hold("grades//AP1", done, hasCache = false)
        // Joined by the student: never dropped either.
        val asked = flights.hold("timetable//AP1", done)
        flights.ask("timetable//AP1")
        settle()
        now += Settings.tunable(Tunable.QueueDrop) * 1000L + 1
        gate.complete(Unit)
        blocker.result.await()
        assertTrue(runCatching { auto.result.await() }.exceptionOrNull() is Dropped)
        assertFalse(auto.started.await())
        assertEquals("exams", user.result.await())
        assertEquals("grades", uncached.result.await())
        assertEquals("timetable", asked.result.await())
        // Dropped is not "just fetched", and the next open fetches again.
        assertFalse(flights.succeededWithin("marks//AP1", 60_000))
        assertFalse(flights.isRunning("marks//AP1"))
        assertFalse(flights.hold("marks//AP1", done).joined)
        assertTrue("dropped marks" in logged)
    }

    @Test
    fun signOutCancelsWaitingRunsToo() = runBlocking {
        Settings.setTunable(Tunable.FetchLanes, 1)
        val flights = lanes()
        val running = flights.hold("profile//AP1", CompletableDeferred())
        val waiting = flights.hold("marks//AP1", CompletableDeferred())
        settle()
        flights.cancelAll()
        assertTrue(running.result.isCancelled)
        assertTrue(waiting.result.isCancelled)
        assertFalse(flights.isRunning("marks//AP1"))
        assertTrue(flights.runningPages.isEmpty())
        val again = flights.start("marks//AP1", "f2") { "fresh" }
        assertFalse(again.joined)
        assertEquals("fresh", again.result.await())
    }

    @Test
    fun slowRecentFetchesRunOneAtATime() = runBlocking {
        val flights = lanes()
        val slow = Settings.tunable(Tunable.SlowFetch) * 1000L + 1
        repeat(2) { flights.fetched("f0", slow, udp = false) }
        assertTrue(flights.slowLink)
        val gates = List(2) { CompletableDeferred<Unit>() }
        val runs = gates.mapIndexed { i, g -> flights.hold("page$i//AP1", g) }
        settle()
        assertEquals(listOf(true, false), runs.map { it.started.isCompleted })
        // Quick again: back to two lanes, and the waiting one starts.
        repeat(2) { flights.fetched("f0", 500, udp = false) }
        assertFalse(flights.slowLink)
        settle()
        assertTrue(runs[1].started.isCompleted)
        assertEquals(1, logged.count { it.startsWith("lanes") })
        // Over UDP: one lane however fast.
        flights.fetched("f0", 500, udp = true)
        assertTrue(flights.slowLink)
        assertEquals(2, logged.count { it.startsWith("lanes") })
        gates.forEach { it.complete(Unit) }
        runs.forEach { it.result.await() }
    }
}
