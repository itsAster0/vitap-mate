package vitapmate.features.login

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.CoreException
import vitapmate.core.Forwarding
import vitapmate.core.ForwardingState
import vitapmate.data.AppCore
import vitapmate.data.AppScope
import vitapmate.data.OtpSource
import vitapmate.data.Settings
import vitapmate.data.askedOtp
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.showToast

/**
 * This phone's mail-forwarding address (Rust's `forwarding.rs`), for Settings → Account → Mail
 * forwarding, its sheet and the OTP prompt's auto-fill. Flows `forwarding.status/setup/check/done/off`,
 * lines `ui.forwarding`, reported to [StateRegistry] as `forwarding`. Never logs the address's
 * keys, codes or mail.
 */
object MailForwarding {
    var status by mutableStateOf<Forwarding?>(null)
        private set

    /** The action running: `setup`, `check`, `done` or `off`. */
    var busy by mutableStateOf<String?>(null)
        private set

    /** The last action's failure (title, message), shown in the sheet, which covers the toasts. */
    var error by mutableStateOf<Pair<String, String>?>(null)
        internal set

    val isOn get() = status?.state?.let { it != ForwardingState.OFF } ?: false

    /** Counts the actions the student starts: a quiet check that began before one is stale. */
    private var actions = 0

    /** Reads the saved status (no network). */
    suspend fun load(trigger: String) {
        try {
            flow("forwarding.status") { id -> show(withContext(Dispatchers.IO) { AppCore.await().forwardingStatus(id) }, id, "trigger" to trigger) }
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
        AppScope.launch { run("setup") { id -> AppCore.await().setUpForwarding(id) }?.let { Settings.otpSource = OtpSource.Forwarding } }
    }

    /** Pulls the latest forwarded mail; [quiet] skips the "nothing yet" toast and error toasts. */
    suspend fun check(quiet: Boolean = false) {
        val before = status
        val after = run("check", quiet) { id -> AppCore.await().checkForwarding(id) } ?: return
        if (!quiet && after.state == ForwardingState.WAITING && after.lastMailAt == before?.lastMailAt) {
            showToast("Nothing yet", "No mail has reached your address in the last 5 minutes. Gmail can take a minute.")
        }
    }

    /** Marks setup done once Gmail's confirmation arrived; no network. */
    fun markDone() {
        AppScope.launch { run("done") { id -> AppCore.await().markForwardingDone(id) } }
    }

    /** Takes this phone off the address; runs on after the sheet closes. */
    fun turnOff() {
        AppScope.launch {
            run("off") { id ->
                AppCore.await().turnOffForwarding(id)
                AppCore.await().forwardingStatus(id)
            }
        }
    }

    /**
     * One action as flow `forwarding.<name>`; a failure is a toast (unless [quiet]), and the status
     * is read again. A [quiet] one (the sheet watching the inbox) doesn't mark the sheet busy, so its
     * buttons don't flicker on each check.
     */
    private suspend fun run(name: String, quiet: Boolean = false, call: suspend (String) -> Forwarding): Forwarding? {
        if (busy != null) return null
        val started = actions
        if (!quiet) {
            actions++
            busy = name
            error = null
            report()
        }
        var flowId = ""
        return try {
            flow("forwarding.$name") { id ->
                flowId = id
                // A quiet check that a "Turn off" overtook would show the address it removed.
                withContext(Dispatchers.IO) { call(id) }.also { if (!quiet || started == actions) show(it, id, "action" to name) }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("ui.forwarding", "$name failed: ${e::class.simpleName}", flowId)
            askedOtp(e)
            if (!quiet) problem(e).let { error = it; showToast(it.first, it.second) }
            load("after_$name")
            null
        } finally {
            if (!quiet) {
                busy = null
                report()
            }
        }
    }

    private fun show(value: Forwarding, flow: String, vararg extra: Pair<String, String>) {
        status = value
        Log.info(
            "ui.forwarding",
            value.state.name.lowercase(),
            flow,
            mapOf("confirmation" to "${value.confirmation != null}", "last_mail_at" to "${value.lastMailAt}", "last_otp_at" to "${value.lastOtpAt}") + extra,
        )
        report()
    }

    private fun report() {
        val s = status
        StateRegistry.report(
            "forwarding",
            mapOf(
                "state" to s?.state?.name,
                "hasAddress" to (s?.address != null),
                "confirmation" to (s?.confirmation != null),
                "lastMailAt" to s?.lastMailAt,
                "lastOtpAt" to s?.lastOtpAt,
                "busy" to busy,
            ),
        )
    }

    private fun problem(e: Throwable): Pair<String, String> = when (e) {
        is CoreException.Offline -> "Couldn't reach the server" to "Check your connection and try again."
        is CoreException.OtpRequired -> "Verify your sign-in" to "VTOP wants the OTP first; enter it, then try again."
        is CoreException.NotSignedIn -> "Sign-in needed" to "Sign in to VTOP, then try again."
        is CoreException.ConsentDenied -> "Not set up yet" to "The server doesn't know this phone yet. Try again while signed in to VTOP."
        is CoreException.VtopUnavailable -> "VTOP is down" to "Try again in a few minutes."
        is CoreException.ForwardingOff -> "Mail forwarding is off" to e.reason
        is CoreException.Other -> "Mail forwarding" to e.reason
        else -> "Something went wrong" to "Try again; if it keeps happening, share the log from the Inspector."
    }
}
