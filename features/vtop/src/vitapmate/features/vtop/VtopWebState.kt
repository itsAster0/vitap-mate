package vitapmate.features.vtop

import vitapmate.core.WebSession
import vitapmate.data.Problem
import vitapmate.data.problemOf

/** Recovery state once a VTOP [WebSession] is active in the WebView. */
internal enum class VtopRecovery {
    /** Ready for a normal cycle: the next bounce to VTOP's login page auto-relogins once. */
    Ready,

    /** Already auto-relogged once without seeing a signed-in page; the next login bounce prompts. */
    AutoRelogin,

    /** Asking the student whether to try signing in again after an automatic re-login bounced. */
    PromptRelogin,
}

/** Explicit lifecycle phase for the in-app VTOP WebView session and crash recovery. */
internal sealed interface VtopWebPhase {
    /** Asking Rust for a confirmed [WebSession]; [requestId] invalidates superseded jobs. */
    data class Preparing(val requestId: Long, val force: Boolean) : VtopWebPhase

    /** Session is ready, waiting for `AndroidView` to attach the freshly created WebView. */
    data class WaitingForView(val session: WebSession, val recovery: VtopRecovery = VtopRecovery.Ready) : VtopWebPhase

    /** WebView is attached and loaded with [session]. */
    data class Active(val session: WebSession, val recovery: VtopRecovery = VtopRecovery.Ready) : VtopWebPhase

    /** Preparing the session failed. */
    data class Failed(val problem: Problem) : VtopWebPhase

    /** Renderer process crashed repeatedly within [VtopWebState.CRASH_COOLDOWN_MS]. */
    data class Crashed(val lastLossMs: Long, val problem: Problem) : VtopWebPhase
}

/** Inputs to [VtopWebState.reduce]. */
internal sealed interface VtopWebEvent {
    data object Open : VtopWebEvent
    data object Retry : VtopWebEvent
    data object OtpResolved : VtopWebEvent
    data class ViewAttached(val generation: Int) : VtopWebEvent
    data class PrepareSucceeded(val requestId: Long, val session: WebSession) : VtopWebEvent
    data class PrepareFailed(val requestId: Long, val problem: Problem) : VtopWebEvent
    data object LoginPageHit : VtopWebEvent
    data object Authenticated : VtopWebEvent
    data object DismissReloginPrompt : VtopWebEvent
    data object ConfirmRelogin : VtopWebEvent
    data class RenderProcessLost(val flow: String) : VtopWebEvent
}

/** Side effects emitted by [VtopWebState.reduce] for [VtopScreen] to execute. */
internal sealed interface VtopWebEffect {
    data object DestroyView : VtopWebEffect
    data object CancelPrepare : VtopWebEffect
    data class Prepare(val requestId: Long, val flowName: String, val force: Boolean) : VtopWebEffect
    data class OpenSession(val session: WebSession) : VtopWebEffect
}

/** Pure reducer output: the next [state] and ordered [effects] to run. */
internal data class VtopWebTransition(
    val state: VtopWebState,
    val effects: List<VtopWebEffect> = emptyList(),
)

/**
 * Pure, clock-injected state machine for the in-app VTOP WebView session, re-login recovery, and
 * renderer-crash handling.
 */
internal data class VtopWebState(
    val phase: VtopWebPhase = VtopWebPhase.Preparing(requestId = 0L, force = false),
    val viewAttached: Boolean = false,
    val viewGeneration: Int = 0,
    val lastLossMs: Long? = null,
    val nextRequestId: Long = 1L,
) {
    fun reduce(event: VtopWebEvent, nowMs: Long): VtopWebTransition = when (event) {
        VtopWebEvent.Open -> startPrepare(FLOW_OPEN, force = false)

        VtopWebEvent.Retry -> when (phase) {
            is VtopWebPhase.Failed, is VtopWebPhase.Crashed -> startPrepare(FLOW_OPEN, force = false)
            else -> noOp()
        }

        VtopWebEvent.OtpResolved -> when (phase) {
            is VtopWebPhase.Failed -> startPrepare(FLOW_OPEN, force = false)
            is VtopWebPhase.Preparing -> if (phase.requestId > 0L) startPrepare(FLOW_OPEN, force = false) else noOp()
            else -> noOp()
        }

        is VtopWebEvent.ViewAttached -> onViewAttached(event.generation)

        is VtopWebEvent.PrepareSucceeded -> onPrepareSucceeded(event.requestId, event.session)

        is VtopWebEvent.PrepareFailed -> onPrepareFailed(event.requestId, event.problem)

        VtopWebEvent.LoginPageHit -> onLoginPageHit()

        VtopWebEvent.Authenticated -> {
            // A signed-in page starts a new recovery cycle; a sign-in alone doesn't (VTOP may bounce straight back).
            if (phase is VtopWebPhase.Active && phase.recovery != VtopRecovery.Ready) {
                VtopWebTransition(copy(phase = phase.copy(recovery = VtopRecovery.Ready)))
            } else {
                noOp()
            }
        }

        VtopWebEvent.DismissReloginPrompt -> {
            if (phase is VtopWebPhase.Active && phase.recovery == VtopRecovery.PromptRelogin) {
                VtopWebTransition(copy(phase = phase.copy(recovery = VtopRecovery.AutoRelogin)))
            } else {
                noOp()
            }
        }

        VtopWebEvent.ConfirmRelogin -> when (phase) {
            is VtopWebPhase.Active -> startPrepare(FLOW_RELOGIN, force = true)
            else -> noOp()
        }

        is VtopWebEvent.RenderProcessLost -> onRenderProcessLost(event.flow, nowMs)
    }

    private fun onViewAttached(generation: Int): VtopWebTransition {
        if (generation != viewGeneration) return noOp()
        val attached = copy(viewAttached = true)
        return when (val current = phase) {
            is VtopWebPhase.WaitingForView -> VtopWebTransition(
                state = attached.copy(phase = VtopWebPhase.Active(current.session, current.recovery)),
                effects = listOf(VtopWebEffect.OpenSession(current.session)),
            )
            else -> VtopWebTransition(attached)
        }
    }

    private fun onPrepareSucceeded(requestId: Long, session: WebSession): VtopWebTransition {
        val current = phase as? VtopWebPhase.Preparing ?: return noOp()
        if (current.requestId == 0L || current.requestId != requestId) return noOp()
        val recovery = if (current.force) VtopRecovery.AutoRelogin else VtopRecovery.Ready
        return if (viewAttached) {
            VtopWebTransition(
                state = copy(phase = VtopWebPhase.Active(session, recovery)),
                effects = listOf(VtopWebEffect.OpenSession(session)),
            )
        } else {
            VtopWebTransition(copy(phase = VtopWebPhase.WaitingForView(session, recovery)))
        }
    }

    private fun onPrepareFailed(requestId: Long, problem: Problem): VtopWebTransition {
        val current = phase as? VtopWebPhase.Preparing ?: return noOp()
        if (current.requestId == 0L || current.requestId != requestId) return noOp()
        return VtopWebTransition(copy(phase = VtopWebPhase.Failed(problem)))
    }

    private fun onLoginPageHit(): VtopWebTransition {
        val current = phase as? VtopWebPhase.Active ?: return noOp()
        return when (current.recovery) {
            VtopRecovery.Ready -> startPrepare(FLOW_RELOGIN, force = true)
            VtopRecovery.AutoRelogin -> VtopWebTransition(copy(phase = current.copy(recovery = VtopRecovery.PromptRelogin)))
            VtopRecovery.PromptRelogin -> noOp()
        }
    }

    private fun onRenderProcessLost(flow: String, nowMs: Long): VtopWebTransition {
        // A page that keeps crashing stops here instead of looping.
        val repeated = lastLossMs != null && (nowMs - lastLossMs) in 0L until CRASH_COOLDOWN_MS
        val reset = copy(viewAttached = false, viewGeneration = viewGeneration + 1, lastLossMs = nowMs)
        if (repeated) {
            val cancel = if (phase is VtopWebPhase.Preparing && phase.requestId > 0L) listOf(VtopWebEffect.CancelPrepare) else emptyList()
            val problem = problemOf(IllegalStateException("VTOP stopped responding"), flow)
            return VtopWebTransition(
                state = reset.copy(phase = VtopWebPhase.Crashed(lastLossMs = nowMs, problem = problem)),
                effects = listOf(VtopWebEffect.DestroyView) + cancel,
            )
        }
        return when (phase) {
            is VtopWebPhase.Active -> reset.startPrepare(FLOW_OPEN, force = false, prefixEffects = listOf(VtopWebEffect.DestroyView))
            is VtopWebPhase.Preparing,
            is VtopWebPhase.WaitingForView,
            is VtopWebPhase.Failed,
            is VtopWebPhase.Crashed,
            -> VtopWebTransition(reset, listOf(VtopWebEffect.DestroyView))
        }
    }

    private fun startPrepare(
        flowName: String,
        force: Boolean,
        prefixEffects: List<VtopWebEffect> = emptyList(),
    ): VtopWebTransition {
        val cancel = if (phase is VtopWebPhase.Preparing && phase.requestId > 0L) listOf(VtopWebEffect.CancelPrepare) else emptyList()
        val requestId = nextRequestId
        return VtopWebTransition(
            state = copy(
                phase = VtopWebPhase.Preparing(requestId = requestId, force = force),
                nextRequestId = requestId + 1L,
            ),
            effects = prefixEffects + cancel + VtopWebEffect.Prepare(requestId, flowName, force),
        )
    }

    private fun noOp() = VtopWebTransition(this)

    companion object {
        const val CRASH_COOLDOWN_MS = 30_000L
        const val FLOW_OPEN = "vtop.open"
        const val FLOW_RELOGIN = "vtop.relogin"
    }
}
