package vitapmate.features.vtop

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue
import vitapmate.core.WebSession
import vitapmate.data.Problem
import vitapmate.data.ProblemKind

class VtopWebStateTest {
    private val session1 = WebSession(baseUrl = "https://vtop.vitap.ac.in", startUrl = "https://vtop.vitap.ac.in/vtop/content")
    private val session2 = WebSession(baseUrl = "https://vtop.vitap.ac.in", startUrl = "https://vtop.vitap.ac.in/vtop/content?2")

    // Rust maps a CoreException (problemOf), and host tests load no Rust library.
    private val offlineProblem = Problem(ProblemKind.OFFLINE, "No Internet Connection", "Check your connection.", "f1", retryable = true)

    private fun openToActive(nowMs: Long = 1_000L): VtopWebState {
        var state = VtopWebState()
        val opened = state.reduce(VtopWebEvent.Open, nowMs)
        state = opened.state
        val prepare = assertIs<VtopWebEffect.Prepare>(opened.effects.single())
        state = state.reduce(VtopWebEvent.ViewAttached(0), nowMs).state
        state = state.reduce(VtopWebEvent.PrepareSucceeded(prepare.requestId, session1), nowMs).state
        assertEquals(VtopWebPhase.Active(session1, VtopRecovery.Ready), state.phase)
        return state
    }

    @Test
    fun initialPrepareOpensSessionWhenViewAlreadyAttached() {
        val opened = VtopWebState().reduce(VtopWebEvent.Open, nowMs = 100L)
        assertEquals(VtopWebPhase.Preparing(requestId = 1L, force = false), opened.state.phase)
        assertEquals(listOf(VtopWebEffect.Prepare(1L, VtopWebState.FLOW_OPEN, force = false)), opened.effects)

        val attached = opened.state.reduce(VtopWebEvent.ViewAttached(generation = 0), nowMs = 120L)
        assertTrue(attached.state.viewAttached)
        assertEquals(VtopWebPhase.Preparing(requestId = 1L, force = false), attached.state.phase)
        assertEquals(emptyList(), attached.effects)

        val ready = attached.state.reduce(VtopWebEvent.PrepareSucceeded(1L, session1), nowMs = 200L)
        assertEquals(VtopWebPhase.Active(session1, VtopRecovery.Ready), ready.state.phase)
        assertEquals(listOf(VtopWebEffect.OpenSession(session1)), ready.effects)
    }

    @Test
    fun initialPrepareWaitsForViewWhenSessionCompletesFirst() {
        val opened = VtopWebState().reduce(VtopWebEvent.Open, nowMs = 100L)
        val waiting = opened.state.reduce(VtopWebEvent.PrepareSucceeded(1L, session1), nowMs = 150L)
        assertEquals(VtopWebPhase.WaitingForView(session1, VtopRecovery.Ready), waiting.state.phase)
        assertEquals(emptyList(), waiting.effects)

        val active = waiting.state.reduce(VtopWebEvent.ViewAttached(generation = 0), nowMs = 180L)
        assertTrue(active.state.viewAttached)
        assertEquals(VtopWebPhase.Active(session1, VtopRecovery.Ready), active.state.phase)
        assertEquals(listOf(VtopWebEffect.OpenSession(session1)), active.effects)
    }

    @Test
    fun rendererCrashWhilePreparingReusesInFlightPrepareAndOpensAfterViewAttaches() {
        val opened = VtopWebState().reduce(VtopWebEvent.Open, nowMs = 1_000L)
        val attached = opened.state.reduce(VtopWebEvent.ViewAttached(0), nowMs = 1_050L)

        // Renderer dies while request 1 is still talking to Rust: destroy view, keep request 1 in flight.
        val lost = attached.state.reduce(VtopWebEvent.RenderProcessLost("f1"), nowMs = 1_100L)
        assertFalse(lost.state.viewAttached)
        assertEquals(1, lost.state.viewGeneration)
        assertEquals(1_100L, lost.state.lastLossMs)
        assertEquals(VtopWebPhase.Preparing(requestId = 1L, force = false), lost.state.phase)
        assertEquals(listOf(VtopWebEffect.DestroyView), lost.effects)

        // Stale ViewAttached from generation 0 is ignored.
        val staleAttach = lost.state.reduce(VtopWebEvent.ViewAttached(0), nowMs = 1_110L)
        assertFalse(staleAttach.state.viewAttached)
        assertEquals(emptyList(), staleAttach.effects)

        // Prepare completes before AndroidView attaches generation 1 -> WaitingForView, not lost.
        val waiting = staleAttach.state.reduce(VtopWebEvent.PrepareSucceeded(1L, session1), nowMs = 1_150L)
        assertEquals(VtopWebPhase.WaitingForView(session1, VtopRecovery.Ready), waiting.state.phase)
        assertEquals(emptyList(), waiting.effects)

        // New WebView attaches -> opens session immediately.
        val active = waiting.state.reduce(VtopWebEvent.ViewAttached(1), nowMs = 1_200L)
        assertTrue(active.state.viewAttached)
        assertEquals(VtopWebPhase.Active(session1, VtopRecovery.Ready), active.state.phase)
        assertEquals(listOf(VtopWebEffect.OpenSession(session1)), active.effects)
    }

    @Test
    fun rendererCrashAfterActiveWaitsForRecreatedViewInsteadOfDroppingSession() {
        val active = openToActive(nowMs = 5_000L)

        val lost = active.reduce(VtopWebEvent.RenderProcessLost("f2"), nowMs = 10_000L)
        assertFalse(lost.state.viewAttached)
        assertEquals(1, lost.state.viewGeneration)
        assertEquals(VtopWebPhase.Preparing(requestId = 2L, force = false), lost.state.phase)
        assertEquals(
            listOf(VtopWebEffect.DestroyView, VtopWebEffect.Prepare(2L, VtopWebState.FLOW_OPEN, force = false)),
            lost.effects,
        )

        // Prepare completes while WebView is still torn down.
        val waiting = lost.state.reduce(VtopWebEvent.PrepareSucceeded(2L, session2), nowMs = 10_050L)
        assertEquals(VtopWebPhase.WaitingForView(session2, VtopRecovery.Ready), waiting.state.phase)
        assertEquals(emptyList(), waiting.effects)

        // Recreated WebView attaches -> opens the prepared session.
        val reopened = waiting.state.reduce(VtopWebEvent.ViewAttached(1), nowMs = 10_100L)
        assertEquals(VtopWebPhase.Active(session2, VtopRecovery.Ready), reopened.state.phase)
        assertEquals(listOf(VtopWebEffect.OpenSession(session2)), reopened.effects)
    }

    @Test
    fun rendererCrashAfterActiveOpensImmediatelyWhenViewAttachesBeforePrepareFinishes() {
        val active = openToActive(nowMs = 5_000L)
        val lost = active.reduce(VtopWebEvent.RenderProcessLost("f2"), nowMs = 10_000L)
        val attached = lost.state.reduce(VtopWebEvent.ViewAttached(1), nowMs = 10_020L)
        assertTrue(attached.state.viewAttached)
        assertEquals(VtopWebPhase.Preparing(requestId = 2L, force = false), attached.state.phase)

        val reopened = attached.state.reduce(VtopWebEvent.PrepareSucceeded(2L, session2), nowMs = 10_050L)
        assertEquals(VtopWebPhase.Active(session2, VtopRecovery.Ready), reopened.state.phase)
        assertEquals(listOf(VtopWebEffect.OpenSession(session2)), reopened.effects)
    }

    @Test
    fun repeatedRendererCrashWithinCooldownStopsInCrashedAndRecoversAfterCooldownOrRetry() {
        val active = openToActive(nowMs = 1_000L)
        val firstLoss = active.reduce(VtopWebEvent.RenderProcessLost("f1"), nowMs = 10_000L)
        assertEquals(VtopWebPhase.Preparing(requestId = 2L, force = false), firstLoss.state.phase)

        // Second crash 29.999 s later while Preparing -> cancels prepare and enters Crashed.
        val secondLoss = firstLoss.state.reduce(VtopWebEvent.RenderProcessLost("f2"), nowMs = 39_999L)
        val crashed = assertIs<VtopWebPhase.Crashed>(secondLoss.state.phase)
        assertEquals(39_999L, crashed.lastLossMs)
        assertEquals("f2", crashed.problem.flow)
        assertEquals(ProblemKind.UNEXPECTED, crashed.problem.kind)
        assertEquals(listOf(VtopWebEffect.DestroyView, VtopWebEffect.CancelPrepare), secondLoss.effects)

        // Stale completion for cancelled request 2 is ignored while Crashed.
        val stale = secondLoss.state.reduce(VtopWebEvent.PrepareSucceeded(2L, session2), nowMs = 40_000L)
        assertEquals(secondLoss.state, stale.state)
        assertEquals(emptyList(), stale.effects)

        // View attaches while Crashed, then user taps Retry.
        val attached = secondLoss.state.reduce(VtopWebEvent.ViewAttached(2), nowMs = 40_100L)
        val retried = attached.state.reduce(VtopWebEvent.Retry, nowMs = 40_200L)
        assertEquals(VtopWebPhase.Preparing(requestId = 3L, force = false), retried.state.phase)
        assertEquals(listOf(VtopWebEffect.Prepare(3L, VtopWebState.FLOW_OPEN, force = false)), retried.effects)

        val activeAgain = retried.state.reduce(VtopWebEvent.PrepareSucceeded(3L, session2), nowMs = 40_500L).state
        // Crash at exact 30 s boundary after lastLossMs (39_999 + 30_000 = 69_999) is no longer repeated.
        val afterCooldown = activeAgain.reduce(VtopWebEvent.RenderProcessLost("f3"), nowMs = 69_999L)
        assertEquals(VtopWebPhase.Preparing(requestId = 4L, force = false), afterCooldown.state.phase)
        assertEquals(
            listOf(VtopWebEffect.DestroyView, VtopWebEffect.Prepare(4L, VtopWebState.FLOW_OPEN, force = false)),
            afterCooldown.effects,
        )
    }

    @Test
    fun stalePrepareCompletionAndFailureAreIgnoredWhenSuperseded() {
        val opened = VtopWebState().reduce(VtopWebEvent.Open, nowMs = 100L)
        val attached = opened.state.reduce(VtopWebEvent.ViewAttached(0), nowMs = 110L)

        // Opened again while request 1 is still in flight -> cancels 1 and starts 2.
        val superseded = attached.state.reduce(VtopWebEvent.Open, nowMs = 150L)
        assertEquals(VtopWebPhase.Preparing(requestId = 2L, force = false), superseded.state.phase)
        assertEquals(
            listOf(VtopWebEffect.CancelPrepare, VtopWebEffect.Prepare(2L, VtopWebState.FLOW_OPEN, force = false)),
            superseded.effects,
        )

        // Stale request 1 succeeding or failing cannot overwrite request 2.
        val staleSuccess = superseded.state.reduce(VtopWebEvent.PrepareSucceeded(1L, session1), nowMs = 160L)
        assertEquals(superseded.state, staleSuccess.state)
        assertEquals(emptyList(), staleSuccess.effects)

        val staleFail = superseded.state.reduce(
            VtopWebEvent.PrepareFailed(1L, offlineProblem),
            nowMs = 170L,
        )
        assertEquals(superseded.state, staleFail.state)
        assertEquals(emptyList(), staleFail.effects)

        // Request 2 succeeds normally.
        val active = superseded.state.reduce(VtopWebEvent.PrepareSucceeded(2L, session2), nowMs = 200L)
        assertEquals(VtopWebPhase.Active(session2, VtopRecovery.Ready), active.state.phase)
        assertEquals(listOf(VtopWebEffect.OpenSession(session2)), active.effects)
    }

    @Test
    fun loginPageAutoReloginsOnceThenPromptsAndAuthenticatedResetsCycle() {
        val active = openToActive(nowMs = 1_000L)

        // First login bounce auto-relogins with force = true.
        val autoRelogin = active.reduce(VtopWebEvent.LoginPageHit, nowMs = 2_000L)
        assertEquals(VtopWebPhase.Preparing(requestId = 2L, force = true), autoRelogin.state.phase)
        assertEquals(listOf(VtopWebEffect.Prepare(2L, VtopWebState.FLOW_RELOGIN, force = true)), autoRelogin.effects)

        // Duplicate LoginPageHit while relogin is in flight is ignored.
        val duplicateWhilePreparing = autoRelogin.state.reduce(VtopWebEvent.LoginPageHit, nowMs = 2_100L)
        assertEquals(autoRelogin.state, duplicateWhilePreparing.state)
        assertEquals(emptyList(), duplicateWhilePreparing.effects)

        // Relogin succeeds -> lands in Active with AutoRelogin recovery.
        val relogged = autoRelogin.state.reduce(VtopWebEvent.PrepareSucceeded(2L, session2), nowMs = 2_500L)
        assertEquals(VtopWebPhase.Active(session2, VtopRecovery.AutoRelogin), relogged.state.phase)
        assertEquals(listOf(VtopWebEffect.OpenSession(session2)), relogged.effects)

        // Second login bounce without seeing an authenticated page -> PromptRelogin dialog.
        val prompt = relogged.state.reduce(VtopWebEvent.LoginPageHit, nowMs = 3_000L)
        assertEquals(VtopWebPhase.Active(session2, VtopRecovery.PromptRelogin), prompt.state.phase)
        assertEquals(emptyList(), prompt.effects)

        // Another hit while prompt is open is a no-op.
        val promptAgain = prompt.state.reduce(VtopWebEvent.LoginPageHit, nowMs = 3_100L)
        assertEquals(prompt.state, promptAgain.state)

        // Dismissing returns to AutoRelogin (next bounce prompts again instead of auto-relogging).
        val dismissed = prompt.state.reduce(VtopWebEvent.DismissReloginPrompt, nowMs = 3_200L)
        assertEquals(VtopWebPhase.Active(session2, VtopRecovery.AutoRelogin), dismissed.state.phase)
        val reprompted = dismissed.state.reduce(VtopWebEvent.LoginPageHit, nowMs = 3_300L)
        assertEquals(VtopWebPhase.Active(session2, VtopRecovery.PromptRelogin), reprompted.state.phase)

        // Confirming prompt triggers forced relogin.
        val manualRelogin = reprompted.state.reduce(VtopWebEvent.ConfirmRelogin, nowMs = 3_400L)
        assertEquals(VtopWebPhase.Preparing(requestId = 3L, force = true), manualRelogin.state.phase)
        assertEquals(listOf(VtopWebEffect.Prepare(3L, VtopWebState.FLOW_RELOGIN, force = true)), manualRelogin.effects)

        val manualActive = manualRelogin.state.reduce(VtopWebEvent.PrepareSucceeded(3L, session1), nowMs = 3_800L).state
        assertEquals(VtopWebPhase.Active(session1, VtopRecovery.AutoRelogin), manualActive.phase)

        // Seeing a signed-in page resets recovery back to Ready, allowing a fresh auto-relogin later.
        val authed = manualActive.reduce(VtopWebEvent.Authenticated, nowMs = 4_000L)
        assertEquals(VtopWebPhase.Active(session1, VtopRecovery.Ready), authed.state.phase)

        val laterBounce = authed.state.reduce(VtopWebEvent.LoginPageHit, nowMs = 5_000L)
        assertEquals(VtopWebPhase.Preparing(requestId = 4L, force = true), laterBounce.state.phase)
        assertEquals(listOf(VtopWebEffect.Prepare(4L, VtopWebState.FLOW_RELOGIN, force = true)), laterBounce.effects)
    }

    @Test
    fun failureAndManualRetryRecoversAndRetryIsIgnoredWhenNotFailed() {
        val opened = VtopWebState().reduce(VtopWebEvent.Open, nowMs = 100L)
        val attached = opened.state.reduce(VtopWebEvent.ViewAttached(0), nowMs = 110L)
        // Retry while Preparing is a no-op.
        assertEquals(emptyList(), attached.state.reduce(VtopWebEvent.Retry, nowMs = 120L).effects)

        val offline = offlineProblem
        val failed = attached.state.reduce(VtopWebEvent.PrepareFailed(1L, offline), nowMs = 200L)
        assertEquals(VtopWebPhase.Failed(offline), failed.state.phase)

        val retried = failed.state.reduce(VtopWebEvent.Retry, nowMs = 300L)
        assertEquals(VtopWebPhase.Preparing(requestId = 2L, force = false), retried.state.phase)
        assertEquals(listOf(VtopWebEffect.Prepare(2L, VtopWebState.FLOW_OPEN, force = false)), retried.effects)

        val active = retried.state.reduce(VtopWebEvent.PrepareSucceeded(2L, session1), nowMs = 400L)
        assertEquals(VtopWebPhase.Active(session1, VtopRecovery.Ready), active.state.phase)
        assertEquals(listOf(VtopWebEffect.OpenSession(session1)), active.effects)

        // Retry while Active is a no-op.
        assertEquals(emptyList(), active.state.reduce(VtopWebEvent.Retry, nowMs = 500L).effects)
    }
}
