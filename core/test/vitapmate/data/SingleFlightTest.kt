package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.CoroutineScope
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
        val first = flights.start("attendance//AP1", "f1", work)
        val tab = launch { first.result.await() }
        yield()
        tab.cancel()
        assertFalse(first.joined)

        // Shown again: it joins the same run, named by the flow that runs it.
        val again = flights.start("attendance//AP1", "f2", work)
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
        assertFalse(flights.start("attendance//AP1", "f4", work).also { it.result.await() }.joined)
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
}
