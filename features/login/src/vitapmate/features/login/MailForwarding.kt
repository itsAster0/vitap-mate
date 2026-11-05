package vitapmate.features.login

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.CoreException
import vitapmate.core.Forwarding
import vitapmate.core.ForwardingState
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.observability.Log
import vitapmate.observability.StateRegistry
import vitapmate.observability.flow
import vitapmate.ui.showToast

/**
 * This phone's mail-forwarding address (Rust's `forwarding.rs`), for Settings → Account → Mail
 * forwarding, its sheet and the OTP prompt's auto-fill. Flows `forwarding.status/setup/check/off`,
 * lines `ui.forwarding`, reported to [StateRegistry] as `forwarding`. Never logs the address's
 * keys, codes or mail.
 */
object MailForwarding {
    var status by mutableStateOf<Forwarding?>(null)
        private set

    /** The action running: `setup`, `check` or `off`. */
    var busy by mutableStateOf<String?>(null)
        private set

    val isOn get() = status?.state?.let { it != ForwardingState.OFF } ?: false

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
    private val scope = MainScope()

    /** Makes or joins the address; runs on after the sheet closes. */
    fun setUp() {
        scope.launch { run("setup") { id -> AppCore.await().setUpForwarding(id) } }
    }

    /** Pulls the latest forwarded mail; [quiet] skips the "nothing yet" toast and error toasts. */
    suspend fun check(quiet: Boolean = false) {
        val before = status
        val after = run("check", quiet) { id -> AppCore.await().checkForwarding(id) } ?: return
        if (!quiet && after.state == ForwardingState.WAITING && after.lastMailAt == before?.lastMailAt) {
            showToast("Nothing yet", "No mail has reached your address in the last 5 minutes. Gmail can take a minute.")
        }
    }

    /** Takes this phone off the address; runs on after the sheet closes. */
    fun turnOff() {
        scope.launch {
            run("off") { id ->
                AppCore.await().turnOffForwarding(id)
                AppCore.await().forwardingStatus(id)
            }
        }
    }

    /** One action as flow `forwarding.<name>`; a failure is a toast (unless [quiet]), and the status is read again. */
    private suspend fun run(name: String, quiet: Boolean = false, call: suspend (String) -> Forwarding): Forwarding? {
        if (busy != null) return null
        busy = name
        report()
        var flowId = ""
        return try {
            flow("forwarding.$name") { id ->
                flowId = id
                withContext(Dispatchers.IO) { call(id) }.also { show(it, id, "action" to name) }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("ui.forwarding", "$name failed: ${e::class.simpleName}", flowId)
            if (e is CoreException.OtpRequired) OtpPrompt.ask(e.reason)
            if (!quiet) problem(e).let { (title, message) -> showToast(title, message) }
            load("after_$name")
            null
        } finally {
            busy = null
            report()
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
        is CoreException.VtopUnavailable -> "VTOP is down" to "Try again in a few minutes."
        is CoreException.Other -> "Mail forwarding" to e.reason
        else -> "Something went wrong" to "Try again; if it keeps happening, share the log from the Inspector."
    }
}
