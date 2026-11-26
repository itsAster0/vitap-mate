package vitapmate.features.login

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import vitapmate.core.ConfirmBy
import vitapmate.core.ConfirmFrom
import vitapmate.core.Forwarding
import vitapmate.core.ForwardingHint
import vitapmate.core.ForwardingReadiness
import vitapmate.core.ForwardingUses
import vitapmate.core.GmailConfirmation
import vitapmate.core.SetupStep

class ForwardingPhaseTest {
    private val off = Forwarding.Off("noreply.sdc@vitap.ac.in", ForwardingReadiness.OFF, ForwardingUses(false, false))
    private val waiting = Forwarding.Waiting("otp+x@vmate.aster0.dev", "noreply.sdc@vitap.ac.in", null, 10L, ForwardingReadiness.NEEDS_FILTER, ForwardingUses(true, true), SetupStep.FILTER, null)
    private val on = Forwarding.On("otp+x@vmate.aster0.dev", "noreply.sdc@vitap.ac.in", 20L, 20L, ForwardingReadiness.READY, ForwardingUses(true, true), null, null, null)
    private val confirm = GmailConfirmation(ConfirmBy.Link("https://mail.google.com/c"), "a@vitapstudent.ac.in", 30L, ConfirmFrom.COLLEGE_INBOX)

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
        assertEquals(
            listOf(ForwardingReadiness.OFF, ForwardingReadiness.OFF, ForwardingReadiness.NEEDS_FILTER, ForwardingReadiness.READY),
            listOf(null.readiness, off.readiness, waiting.readiness, on.readiness),
        )
        assertEquals(listOf(null, false, true), listOf(null.uses, off.uses, waiting.uses).map { it?.usableForAutofill })
    }

    @Test
    fun localWritesRunBesideAnActionWithoutMarkingItBusy() {
        val checking = ForwardingPhase.Busy(ForwardingOp.Check, waiting)
        for (op in listOf(ForwardingOp.Confirm, ForwardingOp.IgnoreConfirm, ForwardingOp.Dismiss, ForwardingOp.MarkDone)) {
            assertTrue(checking.admits(op), op.tag)
            assertNull(ForwardingPhase.Idle(waiting).start(op), op.tag)
        }
        assertFalse(checking.admits(ForwardingOp.Setup))
        assertTrue(ForwardingPhase.Idle(waiting).admits(ForwardingOp.Setup))
    }

    @Test
    fun theWatchFollowsTheAddressAndSkipsRejoining() {
        assertEquals("otp+x@vmate.aster0.dev", waiting.watchedAddress)
        assertEquals("otp+y@vmate.aster0.dev", waiting.copy(address = "otp+y@vmate.aster0.dev").watchedAddress)
        assertNull(waiting.copy(step = SetupStep.REJOINING).watchedAddress)
        assertNull(on.watchedAddress)
        assertNull(null.watchedAddress)
    }

    @Test
    fun aQuietCheckOnReturnOnlyWhenSomethingIsAwaited() {
        assertTrue(waiting.worthQuietCheck)
        assertFalse(waiting.copy(step = SetupStep.REJOINING).worthQuietCheck)
        assertFalse(on.worthQuietCheck)
        assertTrue(on.copy(hint = ForwardingHint.TypedOtp(30L)).worthQuietCheck)
        assertTrue(on.copy(confirmation = confirm).worthQuietCheck)
        assertFalse(off.worthQuietCheck)
        assertFalse(null.worthQuietCheck)
    }
}
