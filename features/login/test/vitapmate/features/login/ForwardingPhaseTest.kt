package vitapmate.features.login

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import vitapmate.core.Forwarding

class ForwardingPhaseTest {
    private val off = Forwarding.Off("noreply.sdc@vitap.ac.in")
    private val waiting = Forwarding.Waiting("otp+x@vmate.aster0.dev", "noreply.sdc@vitap.ac.in", null, 10L)
    private val on = Forwarding.On("otp+x@vmate.aster0.dev", "noreply.sdc@vitap.ac.in", 20L, 20L)

    @Test
    fun oneActionAtATime() {
        val busy = ForwardingPhase.Idle(waiting).start(ForwardingOp.TurnOff)
        assertEquals(ForwardingPhase.Busy(ForwardingOp.TurnOff, waiting), busy)
        assertNull(busy?.start(ForwardingOp.Setup))
        assertNull(busy?.start(ForwardingOp.Check))
    }

    @Test
    fun resultsKeepTheActionUntilItFinishes() {
        val busy = ForwardingPhase.Idle(waiting).start(ForwardingOp.TurnOff)!!
        val shown = busy.shown(off)
        assertEquals(ForwardingPhase.Busy(ForwardingOp.TurnOff, off), shown)
        assertEquals(ForwardingPhase.Idle(off), shown.finish())
        assertEquals(ForwardingPhase.Idle(on), ForwardingPhase.Idle(waiting).shown(on))
    }

    @Test
    fun readsAcrossTheStates() {
        assertFalse(null.hasAddress)
        assertFalse(off.hasAddress)
        assertTrue(waiting.hasAddress)
        assertTrue(on.hasAddress)
        assertEquals(listOf(null, 10L, 20L), listOf(off.lastMailAt, waiting.lastMailAt, on.lastMailAt))
    }
}
