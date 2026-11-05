package vitapmate.features.login

import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import vitapmate.core.CoreException
import vitapmate.core.CoreInterface
import vitapmate.data.AppCore
import vitapmate.data.Settings
import vitapmate.data.Tunable
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow

private const val TIMEOUT_S = 180
private const val DEFAULT_MESSAGE = "VTOP sent a 6-digit code to your registered email."
private const val EXPIRED_MESSAGE = "The code expired. Ask for a new one to continue."

/**
 * One security-OTP prompt (Flutter's `VtopOtpChallenge`, manual entry only) for VTOP's [vtopMessage]:
 * a 3-minute countdown, after which the code is expired and Resend unlocks. A code VTOP turns down,
 * or a network failure, keeps the prompt open with the reason; anything else is thrown to the
 * caller. Flows are `<prefix>.submit_otp` / `<prefix>.resend_otp` and lines go to [src]; the sheet
 * adds `<prefix>.minimize_otp` / `<prefix>.reopen_otp` (src `ui.otp`).
 * With mail forwarding on, [autofill] fills in the forwarded code. TODO: Gmail auto-fill and auto-resend.
 */
@Stable
class OtpChallenge(vtopMessage: String, val prefix: String, private val src: String) {
    var code by mutableStateOf("")
    var message by mutableStateOf(vtopMessage.trim().ifEmpty { DEFAULT_MESSAGE })
        private set
    var error by mutableStateOf<String?>(null)
        private set
    var remaining by mutableIntStateOf(TIMEOUT_S)
        private set
    var submitting by mutableStateOf(false)
        private set
    var resending by mutableStateOf(false)
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

    /** Unix seconds when VTOP sent this round's code (about when the prompt opened). */
    private var roundStart = System.currentTimeMillis() / 1000

    /** Bumps on each resend; key the countdown's effect on it. */
    var round by mutableIntStateOf(0)
        private set

    val canResend get() = remaining == 0 && !submitting && !resending
    val canSubmit get() = !submitting && !resending && code.length == 6

    /** The countdown; run it in a `LaunchedEffect(challenge, challenge.round)`. */
    suspend fun countDown() {
        while (remaining > 0) {
            delay(1000)
            if (remaining == 0) return // a rejected, expired code stopped it
            if (--remaining == 0) {
                Log.info(src, "otp expired")
                message = EXPIRED_MESSAGE
                error = "This code has expired."
            }
        }
    }

    /** True once VTOP accepts the code; false when it turned it down or couldn't be reached. */
    suspend fun submit(core: CoreInterface): Boolean {
        if (!canSubmit) return false
        submitting = true
        error = null
        try {
            flow("$prefix.submit_otp") { id ->
                lastFlow = id
                core.submitOtp(id, code)
            }
            Log.info(src, "otp accepted", lastFlow)
            return true
        } catch (e: CoreException.OtpRejected) {
            val reason = e.reason.trim().ifEmpty { "VTOP didn't accept this code." }
            val expired = reason.contains("expire", ignoreCase = true)
            Log.info(src, "otp rejected", lastFlow, mapOf("expired" to "$expired"))
            rejected += code
            minimized = false // the student sees why and can type a code
            if (expired) {
                remaining = 0
                message = EXPIRED_MESSAGE
            }
            minimized = false
            error = reason
            return false
        } catch (e: CoreException) {
            if (e !is CoreException.Offline && e !is CoreException.VtopUnavailable) throw e
            error = problemOf(e, lastFlow).message
            return false
        } finally {
            submitting = false
        }
    }

    suspend fun resend(core: CoreInterface) {
        if (!canResend) return
        resending = true
        error = null
        try {
            flow("$prefix.resend_otp") { id -> core.resendOtp(id) }
            Log.info(src, "otp resent")
            remaining = TIMEOUT_S
            code = ""
            roundStart = System.currentTimeMillis() / 1000
            message = "VTOP sent a new code to your registered email."
            round++
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            error = (e as? CoreException.OtpRejected)?.reason?.trim()?.ifEmpty { null } ?: problemOf(e, "").message
        } finally {
            resending = false
        }
    }

    /**
     * With mail forwarding on, pulls the forwarded OTP every [Tunable.OtpPoll] (2 s) while the code is live
     * (flow `forwarding.poll` per run, each pull at most [PULL_TIMEOUT_MS]); a new code sent this
     * round is filled in and [onCode] runs. The prompt stays collapsed to its pill while it waits,
     * and opens when no code came or VTOP rejected it. Run it in a `LaunchedEffect(challenge, challenge.round)`,
     * which cancels it. Never logs the code.
     */
    suspend fun autofill(onCode: () -> Unit) {
        MailForwarding.load("otp")
        if (!MailForwarding.isOn) return
        source = "Watching your forwarded mail…"
        // The forwarded code fills in by itself: stay a pill until it comes or the wait ends.
        minimized = true
        flow("forwarding.poll") { id ->
            Log.info(src, "waiting for a forwarded otp", id, mapOf("since" to "$roundStart"))
            var pulls = 0
            // Polling goes on after a fill: the box can hold an older code (sent up to a minute
            // before this round) that VTOP turns down, and the right one comes a moment later.
            val filled = mutableSetOf<String>()
            while (remaining > 0) {
                val found = try {
                    withTimeoutOrNull(PULL_TIMEOUT_MS) { withContext(Dispatchers.IO) { AppCore.await().fetchForwardedOtp(id, roundStart) } }
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Throwable) {
                    Log.warn(src, "forwarded otp pull failed: ${e::class.simpleName}", id)
                    null
                }
                pulls++
                if (found != null && found !in rejected && found !in filled && !submitting) {
                    filled += found
                    code = found
                    source = "From your forwarded mail"
                    Log.info(src, "forwarded otp filled", id, mapOf("pulls" to "$pulls"))
                    onCode()
                }
                delay(Settings.tunable(Tunable.OtpPoll) * 1000L)
            }
            Log.info(src, if (filled.isEmpty()) "no forwarded otp" else "no forwarded otp accepted", id, mapOf("pulls" to "$pulls", "filled" to "${filled.size}"))
            source = null
            minimized = false
        }
    }
}

private const val PULL_TIMEOUT_MS = 8_000L
