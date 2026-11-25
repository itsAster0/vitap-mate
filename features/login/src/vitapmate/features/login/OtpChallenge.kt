package vitapmate.features.login

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.CoreException
import vitapmate.core.CoreInterface
import vitapmate.core.LiveTopic
import vitapmate.core.OtpRejection
import vitapmate.data.AppCore
import vitapmate.data.AppScope
import vitapmate.data.Gmail
import vitapmate.data.LiveWake
import vitapmate.data.OtpSource
import vitapmate.data.Settings
import vitapmate.data.Tunable
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow

private const val TIMEOUT_S = OtpAutofill.LIFETIME_S
private const val DEFAULT_MESSAGE = "VTOP sent a 6-digit code to your registered email."
private const val EXPIRED_MESSAGE = "The code expired. Ask for a new one to continue."

/** Lifecycle of one OTP prompt round. */
sealed interface OtpPhase {
    data class Active(val remaining: Int) : OtpPhase
    data class Submitting(val remaining: Int) : OtpPhase
    data object Expired : OtpPhase
    data object Resending : OtpPhase
    data object Done : OtpPhase
}

/**
 * One security-OTP prompt (Flutter's `VtopOtpChallenge`, manual entry only) for VTOP's [vtopMessage]
 * and the code Rust says was mailed at [issuedAt] (unix seconds):
 * a 3-minute countdown, after which the code is expired and Resend unlocks. A code VTOP turns down,
 * or a network failure, keeps the prompt open with the reason; anything else is thrown to the
 * caller. Flows are `<prefix>.submit_otp` / `<prefix>.resend_otp` and lines go to [src]; the sheet
 * adds `<prefix>.minimize_otp` / `<prefix>.reopen_otp` (src `ui.otp`).
 * With mail forwarding or Gmail set up ([OtpAutofill]), [autofill] fills in the code and
 * [countDown] asks VTOP for a new code once if the first round expires without one.
 */
@Stable
class OtpChallenge(vtopMessage: String, issuedAt: Long, val prefix: String, private val src: String) {
    private val phaseFlow = MutableStateFlow<OtpPhase>(OtpPhase.Active(TIMEOUT_S))
    var phase: OtpPhase by mutableStateOf(OtpPhase.Active(TIMEOUT_S))
        private set

    var code by mutableStateOf("")
    var message by mutableStateOf(vtopMessage.trim().ifEmpty { DEFAULT_MESSAGE })
        private set
    var error by mutableStateOf<String?>(null)
        private set

    /** Collapsed into the pill; the code, countdown and error stay. */
    var minimized by mutableStateOf(false)

    /** The flow of the last submit, for a [vitapmate.data.Problem] made from what it threw. */
    var lastFlow = ""
        private set

    /** What filled in the code for the student, or what is being watched for it. */
    var source by mutableStateOf<String?>(null)
        private set

    /** Codes VTOP turned down: auto-fill skips them. */
    private val rejected = mutableSetOf<String>()

    /**
     * When this round's code was mailed (unix seconds), as Rust tells it: [resend] takes the new
     * one, and the fetch sheet passes on a fresh sign-in's under the same prompt. Auto-fill reads
     * mail from then on.
     */
    var issuedAt by mutableLongStateOf(issuedAt)

    /** Bumps on each resend; key the countdown's effect on it. */
    var round by mutableIntStateOf(0)
        private set

    /** At most one automatic resend per challenge when auto-fill is active and the first round expires. */
    private var autoResent = false

    val remaining: Int
        get() = when (val p = phase) {
            is OtpPhase.Active -> p.remaining
            is OtpPhase.Submitting -> p.remaining
            OtpPhase.Expired, OtpPhase.Resending, OtpPhase.Done -> 0
        }

    val submitting: Boolean get() = phase is OtpPhase.Submitting
    val resending: Boolean get() = phase is OtpPhase.Resending
    val canResend: Boolean get() = phase is OtpPhase.Expired
    val canSubmit: Boolean get() = (phase is OtpPhase.Active || phase is OtpPhase.Expired) && code.length == 6

    private val willAutoResend get() = !autoResent && OtpAutofill.inUse() != null

    private fun transition(next: OtpPhase) {
        phase = next
        phaseFlow.value = next
    }

    /**
     * The countdown; run it in a `LaunchedEffect(challenge, challenge.round)`. When OTP auto-fill
     * is active and the first round expires without an accepted code, calls [resend] once so a
     * delayed or lost first email doesn't stall sign-in.
     */
    suspend fun countDown(core: (suspend () -> CoreInterface)? = null) {
        while (remaining > 0) {
            delay(1000)
            when (val p = phase) {
                is OtpPhase.Active -> {
                    val left = p.remaining - 1
                    if (left <= 0) {
                        transition(OtpPhase.Expired)
                        Log.info(src, "otp expired")
                        message = EXPIRED_MESSAGE
                        error = "This code has expired."
                    } else {
                        transition(OtpPhase.Active(left))
                    }
                }
                is OtpPhase.Submitting -> {
                    val left = (p.remaining - 1).coerceAtLeast(0)
                    transition(OtpPhase.Submitting(left))
                    if (left == 0) {
                        Log.info(src, "otp expired")
                        message = EXPIRED_MESSAGE
                    }
                }
                OtpPhase.Expired, OtpPhase.Resending, OtpPhase.Done -> break
            }
        }
        // Wait for any in-flight verification to settle; if accepted, phase becomes Done.
        phaseFlow.first { it !is OtpPhase.Submitting }
        if (canResend && !autoResent && core != null && OtpAutofill.inUse() != null) {
            autoResent = true
            Log.info(src, "otp auto-resend after expiry")
            try {
                resend(core())
            } catch (e: CancellationException) {
                throw e
            } catch (e: Throwable) {
                error = problemOf(e, "").message
            }
            if (remaining == 0) minimized = false
        }
    }

    /** True once VTOP accepts the code; false when it turned it down or couldn't be reached. */
    suspend fun submit(core: CoreInterface): Boolean {
        if (!canSubmit) return false
        transition(OtpPhase.Submitting(remaining))
        error = null
        var accepted = false
        var forceExpired = false
        try {
            flow("$prefix.submit_otp") { id ->
                lastFlow = id
                core.submitOtp(id, code)
            }
            Log.info(src, "otp accepted", lastFlow)
            accepted = true
            transition(OtpPhase.Done)
            gmailOtp?.takeIf { it.code == code }?.let { otp ->
                val flowId = lastFlow
                // Gmail clean-up runs after the prompt has closed.
                AppScope.launch { Gmail.tidyUp(otp, flowId) }
            }
            return true
        } catch (e: CoreException.OtpRejected) {
            val reason = e.reason.trim().ifEmpty { "VTOP didn't accept this code." }
            val expired = e.kind == OtpRejection.EXPIRED
            Log.info(src, "otp rejected", lastFlow, mapOf("kind" to e.kind.name.lowercase()))
            rejected += code
            minimized = false // the student sees why and can type a code
            if (expired) {
                forceExpired = true
                message = EXPIRED_MESSAGE
            }
            error = reason
            return false
        } catch (e: CoreException) {
            if (e !is CoreException.Offline && e !is CoreException.VtopUnavailable) throw e
            error = problemOf(e, lastFlow).message
            return false
        } finally {
            if (!accepted && phase is OtpPhase.Submitting) {
                val left = if (forceExpired) 0 else remaining
                transition(if (left <= 0) OtpPhase.Expired else OtpPhase.Active(left))
            }
        }
    }

    suspend fun resend(core: CoreInterface) {
        if (!canResend) return
        transition(OtpPhase.Resending)
        error = null
        var resent = false
        try {
            val sentAt = flow("$prefix.resend_otp") { id -> core.resendOtp(id) }
            Log.info(src, "otp resent")
            resent = true
            code = ""
            // None: signing in afresh needed no code, so there is no newer mail to wait for.
            sentAt?.let { issuedAt = it.toLong() }
            message = "VTOP sent a new code to your registered email."
            transition(OtpPhase.Active(TIMEOUT_S))
            round++
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            error = (e as? CoreException.OtpRejected)?.reason?.trim()?.ifEmpty { null } ?: problemOf(e, "").message
        } finally {
            if (!resent && phase is OtpPhase.Resending) {
                transition(OtpPhase.Expired)
            }
        }
    }

    /** The Gmail message the filled-in code came from, trashed (or marked read) once VTOP accepts it. */
    private var gmailOtp: Gmail.Otp? = null

    /**
     * Fills in the code by itself from the one [OtpAutofill.inUse]: the forwarded OTP (flow
     * `forwarding.poll`) or VTOP's mail in Gmail (flow `gmail.poll`).
     * Each pull runs every [Tunable.OtpPoll] (2 s) while the code is live, at most
     * [PULL_TIMEOUT_MS]; a new code sent this round is filled in and [onCode] runs. The prompt
     * stays collapsed to its pill while it waits, and opens when no code came or VTOP rejected
     * it. Run it in a `LaunchedEffect(challenge, challenge.round)`, which cancels it. Never logs the code.
     */
    suspend fun autofill(onCode: () -> Unit) {
        MailForwarding.load("otp")
        when (OtpAutofill.inUse()) {
            OtpSource.Forwarding -> watch("forwarding.poll", "forwarded mail", onCode, LiveTopic.MAIL_INBOX) { id -> AppCore.await().fetchForwardedOtp(id, issuedAt) }
            OtpSource.Gmail -> {
                val tried = mutableSetOf<String>()
                watch("gmail.poll", "Gmail", onCode) { id ->
                    Gmail.findOtp(issuedAt, tried, id)?.let { otp ->
                        tried += otp.messageId
                        gmailOtp = otp
                        otp.code
                    }
                }
            }
            null -> if (OtpAutofill.selected() == OtpSource.Gmail && Gmail.account?.needsReconnect == true) source = "Gmail needs reconnecting: Settings → OTP auto-fill"
        }
    }

    private suspend fun watch(flowName: String, where: String, onCode: () -> Unit, wakeOn: LiveTopic? = null, pull: suspend (String) -> String?) = coroutineScope {
        source = "Watching your $where…"
        // The code fills in by itself: stay a pill until it comes or the wait ends.
        minimized = true
        flow(flowName) { id ->
            Log.info(src, "waiting for an otp from $where", id, mapOf("since" to "$issuedAt"))
            var pulls = 0
            // Polling goes on after a fill: an older code (sent up to a minute before this
            // round) can come first and be turned down, and the right one comes a moment later.
            val filled = mutableSetOf<String>()
            val wake = wakeOn?.let { LiveWake(this@coroutineScope, it) }
            var off = false
            try {
                while (remaining > 0 && phase !is OtpPhase.Done) {
                    val found = try {
                        withTimeoutOrNull(PULL_TIMEOUT_MS) { withContext(Dispatchers.IO) { pull(id) } }
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Gmail.GmailException) {
                        Log.warn(src, "gmail pull failed: ${e.error.id}", id)
                        // Offline is worth another try; a revoked grant or a refusal isn't.
                        if (e.error !is Gmail.GmailError.Network) {
                            error = e.message
                            break
                        }
                        null
                    } catch (e: CoreException.ForwardingOff) {
                        Log.info(src, "forwarding is off", id)
                        error = e.reason
                        off = true
                        break
                    } catch (e: Throwable) {
                        Log.warn(src, "otp pull failed: ${e::class.simpleName}", id)
                        null
                    }
                    pulls++
                    if (found != null && found !in rejected && found !in filled && !submitting) {
                        filled += found
                        code = found
                        source = "From your $where"
                        Log.info(src, "otp filled from $where", id, mapOf("pulls" to "$pulls"))
                        onCode()
                    }
                    val interval = Settings.tunable(Tunable.OtpPoll) * 1000L
                    if (wake != null) wake.next(interval) else delay(interval)
                }
            } finally {
                wake?.stop()
            }
            Log.info(src, if (filled.isEmpty()) "no otp from $where" else "no otp from $where accepted", id, mapOf("pulls" to "$pulls", "filled" to "${filled.size}"))
            source = null
            // Nothing will come this round or the next: open, so the student types the code.
            if (off || (!resending && !willAutoResend)) minimized = false
            // Settings shows forwarding as it is now (forgotten when the address is gone).
            if (off) MailForwarding.load("forwarding_off")
        }
    }
}

private const val PULL_TIMEOUT_MS = 8_000L
