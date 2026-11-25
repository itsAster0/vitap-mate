package vitapmate.features.login

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import vitapmate.core.CoreException
import vitapmate.core.Forwarding
import vitapmate.core.ForwardingReadiness
import vitapmate.data.AppCore
import vitapmate.data.AppScope
import vitapmate.data.OtpSource
import vitapmate.data.Settings
import vitapmate.data.askedOtp
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.showToast

/** An action the student starts on the address; [tag] names its flow, `forwarding.<tag>`. */
enum class ForwardingOp(val tag: String) { Setup("setup"), Check("check"), MarkDone("done"), TurnOff("off") }

/** The sheet's view of the address: the last status Rust gave, and the student's action running, if any. */
sealed interface ForwardingPhase {
    val status: Forwarding?

    data class Idle(override val status: Forwarding?) : ForwardingPhase

    data class Busy(val op: ForwardingOp, override val status: Forwarding?) : ForwardingPhase

    /** [op] starting; null while another runs (one at a time). */
    fun start(op: ForwardingOp): Busy? = when (this) {
        is Idle -> Busy(op, status)
        is Busy -> null
    }

    /** [value] read from Rust, keeping the action running. */
    fun shown(value: Forwarding): ForwardingPhase = when (this) {
        is Idle -> Idle(value)
        is Busy -> Busy(op, value)
    }

    fun finish(): Idle = Idle(status)
}

/** How far setup has progressed, as decided by Rust. */
val Forwarding?.readiness: ForwardingReadiness
    get() = when (this) {
        null -> ForwardingReadiness.OFF
        is Forwarding.Off -> readiness
        is Forwarding.Waiting -> readiness
        is Forwarding.On -> readiness
    }

/** Whether the phone has an address, set up or not. */
val Forwarding?.hasAddress get() = readiness != ForwardingReadiness.OFF

/** Unix seconds of the latest forwarded mail of either kind. */
val Forwarding.lastMailAt get() = when (this) {
    is Forwarding.Off -> null
    is Forwarding.Waiting -> lastMailAt
    is Forwarding.On -> lastMailAt
}

/**
 * This phone's mail-forwarding address (Rust's `forwarding.rs`), for Settings → Account → Mail
 * forwarding, its sheet and the OTP prompt's auto-fill. Flows `forwarding.status/setup/check/done/off`,
 * lines `ui.forwarding`, reported to [StateRegistry] as `forwarding`. Never logs the address's
 * keys, codes or mail.
 */
object MailForwarding {
    /** Compose state, written on the main thread only ([load], [poll] and [run] go there), so start-if-idle is atomic. */
    var phase by mutableStateOf<ForwardingPhase>(ForwardingPhase.Idle(null))
        private set

    val status get() = phase.status

    /** The student's action running, if any. */
    val busy get() = (phase as? ForwardingPhase.Busy)?.op

    /** The last action's failure (title, message), shown in the sheet, which covers the toasts. */
    var error by mutableStateOf<Pair<String, String>?>(null)
        internal set

    val readiness get() = status.readiness

    val isOn get() = status.hasAddress

    /** Held by each call into Rust that changes the address: a quiet check never runs beside an action. */
    private val calls = Mutex()

    /** Reads the saved status (no network); while an action runs, its own result shows instead. */
    suspend fun load(trigger: String): Unit = withContext(Dispatchers.Main.immediate) {
        try {
            flow("forwarding.status") { id ->
                val value = withContext(Dispatchers.IO) { AppCore.await().forwardingStatus(id) }
                if (phase is ForwardingPhase.Idle) show(value, id, "trigger" to trigger)
            }
        } catch (e: CancellationException) {
            throw e
        } catch (_: Throwable) {
            // The core failed to start: logged there.
        }
    }

    // Outlives the sheet: closing it mid-way would drop Rust's call between edge requests, leaving
    // an address made on the edge but not saved here (or not tied to the student), or half removed.

    /** Makes or joins the address; runs on after the sheet closes. */
    fun setUp() {
        AppScope.launch { run(ForwardingOp.Setup) { id -> AppCore.await().setUpForwarding(id) }?.let { Settings.otpSource = OtpSource.Forwarding } }
    }

    /**
     * Pulls the latest forwarded mail. A [quiet] one (the sheet watching the inbox) is skipped
     * while an action runs, doesn't mark the sheet busy (so its buttons don't flicker on each
     * check) and shows no toasts.
     */
    suspend fun check(quiet: Boolean = false) {
        if (quiet) return poll()
        val before = status?.lastMailAt
        val after = run(ForwardingOp.Check) { id -> AppCore.await().checkForwarding(id) } ?: return
        if (after is Forwarding.Waiting && after.lastMailAt == before) {
            showToast("Nothing yet", "No mail has reached your address in the last 5 minutes. Gmail can take a minute.")
        }
    }

    /** Marks setup done once Gmail's confirmation arrived; no network. */
    fun markDone() {
        AppScope.launch { run(ForwardingOp.MarkDone) { id -> AppCore.await().markForwardingDone(id) } }
    }

    /** Takes this phone off the address; runs on after the sheet closes. */
    fun turnOff() {
        AppScope.launch {
            run(ForwardingOp.TurnOff) { id ->
                AppCore.await().turnOffForwarding(id)
                AppCore.await().forwardingStatus(id)
            }
        }
    }

    private suspend fun poll(): Unit = withContext(Dispatchers.Main.immediate) {
        if (phase is ForwardingPhase.Busy || !calls.tryLock()) return@withContext
        var flowId = ""
        try {
            flow("forwarding.check") { id ->
                flowId = id
                val value = withContext(Dispatchers.IO) { AppCore.await().checkForwarding(id) }
                // An action started meanwhile waits for this call, then shows its own result.
                if (phase is ForwardingPhase.Idle) show(value, id, "action" to ForwardingOp.Check.tag)
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("ui.forwarding", "check failed: ${e::class.simpleName}", flowId)
            askedOtp(e)
            load("after_check")
        } finally {
            calls.unlock()
        }
    }

    /** [op] as flow `forwarding.<tag>`, after any quiet check; a failure is a toast, and the status is read again. */
    private suspend fun run(op: ForwardingOp, call: suspend (String) -> Forwarding): Forwarding? = withContext(Dispatchers.Main.immediate) {
        phase = phase.start(op) ?: return@withContext null
        error = null
        report()
        var flowId = ""
        var failed = false
        val value = try {
            calls.withLock {
                flow("forwarding.${op.tag}") { id ->
                    flowId = id
                    withContext(Dispatchers.IO) { call(id) }.also { show(it, id, "action" to op.tag) }
                }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("ui.forwarding", "${op.tag} failed: ${e::class.simpleName}", flowId)
            askedOtp(e)
            problem(e).let { error = it; showToast(it.first, it.second) }
            failed = true
            null
        } finally {
            phase = phase.finish()
            report()
        }
        if (failed) load("after_${op.tag}")
        value
    }

    private fun show(value: Forwarding, flow: String, vararg extra: Pair<String, String>) {
        phase = phase.shown(value)
        val (state, fields) = when (value) {
            is Forwarding.Off -> "off" to mapOf()
            is Forwarding.Waiting -> "waiting" to mapOf("confirmation" to "${value.confirmation != null}", "last_mail_at" to "${value.lastMailAt}")
            is Forwarding.On -> "on" to mapOf("last_mail_at" to "${value.lastMailAt}", "last_otp_at" to "${value.lastOtpAt}")
        }
        Log.info("ui.forwarding", state, flow, fields + extra)
        report()
    }

    private fun report() {
        val s = status
        StateRegistry.report(
            "forwarding",
            mapOf(
                "state" to s?.let { it::class.simpleName?.uppercase() },
                "hasAddress" to s.hasAddress,
                "confirmation" to ((s as? Forwarding.Waiting)?.confirmation != null),
                "lastMailAt" to s?.lastMailAt,
                "lastOtpAt" to (s as? Forwarding.On)?.lastOtpAt,
                "busy" to busy?.tag,
            ),
        )
    }

    private fun problem(e: Throwable): Pair<String, String> = when (e) {
        is CoreException.Offline -> "Couldn't reach the server" to "Check your connection and try again."
        is CoreException.OtpRequired -> "Verify your sign-in" to "VTOP wants the OTP first; enter it, then try again."
        is CoreException.NotSignedIn -> "Sign-in needed" to "Sign in to VTOP, then try again."
        is CoreException.ConsentDenied -> "Not set up yet" to "The server doesn't know this phone yet. Try again while signed in to VTOP."
        is CoreException.RegnoNotShared -> "Not shared" to "Mail forwarding needs your registration number on the vitapmate server."
        is CoreException.VtopUnavailable -> "VTOP is down" to "Try again in a few minutes."
        is CoreException.ForwardingOff -> "Mail forwarding is off" to e.reason
        is CoreException.Other -> "Mail forwarding" to e.reason
        else -> "Something went wrong" to "Try again; if it keeps happening, share the log from the Inspector."
    }
}
