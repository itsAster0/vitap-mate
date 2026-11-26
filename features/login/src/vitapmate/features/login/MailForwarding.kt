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
import vitapmate.core.ForwardingUses
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
 * An action the student starts on the address; [tag] names its flow, `forwarding.<tag>`. A [local]
 * one only writes the phone's own progress file (Rust orders it with the rest), so it never waits
 * for, or is turned away by, an action on the network.
 */
enum class ForwardingOp(val tag: String, val local: Boolean = false) {
    Setup("setup"),
    Check("check"),
    MarkDone("done", local = true),
    Confirm("confirm", local = true),
    IgnoreConfirm("ignore", local = true),
    Dismiss("dismiss", local = true),
    TurnOff("off"),
    Delete("delete"),
}

/** What the sheet says about the last action, in the sheet itself (the toasts sit under it). */
sealed interface ForwardingFeedback {
    val title: String
    val message: String

    data class Failed(override val title: String, override val message: String) : ForwardingFeedback

    data class Note(override val title: String, override val message: String) : ForwardingFeedback
}

/** The sheet's view of the address: the last status Rust gave, and the student's action running, if any. */
sealed interface ForwardingPhase {
    val status: Forwarding?

    data class Idle(override val status: Forwarding?) : ForwardingPhase

    data class Busy(val op: ForwardingOp, override val status: Forwarding?) : ForwardingPhase

    /** Network [op] starting; null while another runs (one at a time), and for a local one, which never marks the sheet busy. */
    fun start(op: ForwardingOp): Busy? = when {
        op.local -> null
        this is Idle -> Busy(op, status)
        else -> null
    }

    /** Whether [op] may run now: a local one always does. */
    fun admits(op: ForwardingOp): Boolean = op.local || this is Idle

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

/** What forwarding is good for now (OTP auto-fill, background sync), as decided by Rust; null before it loaded. */
val Forwarding?.uses: ForwardingUses?
    get() = when (this) {
        null -> null
        is Forwarding.Off -> uses
        is Forwarding.Waiting -> uses
        is Forwarding.On -> uses
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
 * forwarding, its sheet and the OTP prompt's auto-fill. Flows `forwarding.status/setup/check/done/off/delete`,
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

    /** The last action's failure or note, shown in the sheet, which covers the toasts. */
    var feedback by mutableStateOf<ForwardingFeedback?>(null)
        internal set

    /** Bumped on every local write, so an older in-flight read/poll/action never stomps its result. */
    private var writeSeq = 0

    val readiness get() = status.readiness

    /** Whether the OTP prompt fills in forwarded codes (Rust's `ForwardingUses`). */
    val usableForAutofill get() = status.uses?.usableForAutofill == true

    /** Whether background sync may sign in with forwarded codes (Rust's `ForwardingUses`). */
    val readyForSync get() = status.uses?.readyForSync == true

    val isOn get() = status.hasAddress

    /** Held by each call into Rust that hits the network: a quiet check never runs beside an action. */
    private val calls = Mutex()

    /** Orders local progress writes with each other without ever waiting behind network [calls]. */
    private val writes = Mutex()

    /** Reads the saved status (no network); while an action runs, its own result shows instead. */
    suspend fun load(trigger: String): Unit = withContext(Dispatchers.Main.immediate) {
        val seq = writeSeq
        try {
            flow("forwarding.status") { id ->
                val value = withContext(Dispatchers.IO) { AppCore.await().forwardingStatus(id) }
                if (phase is ForwardingPhase.Idle && writeSeq == seq) show(value, id, "trigger" to trigger)
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
            note("Nothing yet", "No mail has reached your address in the last 5 minutes. Gmail can take a minute.")
        }
    }

    /** Marks setup done once Gmail's confirmation arrived; no network. */
    fun markDone() {
        AppScope.launch { write(ForwardingOp.MarkDone) { id -> AppCore.await().markForwardingDone(id) } }
    }

    /** The student opened Gmail's confirm link: the sheet moves on to the filter. No network. */
    fun openedConfirm() {
        AppScope.launch { write(ForwardingOp.Confirm) { id -> AppCore.await().openedForwardingConfirm(id) } }
    }

    /** Sets the shown Gmail confirmation aside (another Gmail asked, say). No network. */
    fun ignoreConfirm() {
        AppScope.launch { write(ForwardingOp.IgnoreConfirm) { id -> AppCore.await().ignoreForwardingConfirm(id) } }
    }

    /** Hides the typed-OTP hint until another OTP is typed. No network. */
    fun dismissHint() {
        AppScope.launch { write(ForwardingOp.Dismiss) { id -> AppCore.await().dismissForwardingHint(id) } }
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

    /** Deletes the address for every phone on it; runs on after the sheet closes. */
    fun deleteEverywhere() {
        AppScope.launch {
            run(ForwardingOp.Delete) { id ->
                AppCore.await().deleteForwardingEverywhere(id)
                AppCore.await().forwardingStatus(id)
            }
        }
    }

    private suspend fun poll(): Unit = withContext(Dispatchers.Main.immediate) {
        if (phase is ForwardingPhase.Busy || !calls.tryLock()) return@withContext
        val seq = writeSeq
        var flowId = ""
        var reread = false
        try {
            flow("forwarding.check") { id ->
                flowId = id
                val value = withContext(Dispatchers.IO) { AppCore.await().checkForwarding(id) }
                // An action started meanwhile waits for this call, then shows its own result;
                // a local write landed meanwhile needs the merged status reread instead of this older one.
                if (phase is ForwardingPhase.Idle) {
                    if (writeSeq == seq) show(value, id, "action" to ForwardingOp.Check.tag) else reread = true
                }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("ui.forwarding", "check failed: ${e::class.simpleName}", flowId)
            askedOtp(e)
            reread = true
        } finally {
            calls.unlock()
        }
        if (reread) load("after_check")
    }

    /** A note for the sheet (and a toast for when it's closed). */
    internal fun note(title: String, message: String) {
        feedback = ForwardingFeedback.Note(title, message)
        showToast(title, message)
    }

    /** A failure for the sheet (and a toast for when it's closed). */
    internal fun failed(title: String, message: String) {
        feedback = ForwardingFeedback.Failed(title, message)
        showToast(title, message)
    }

    /**
     * Local [op] as flow `forwarding.<tag>`: never dropped or queued behind [calls], as Rust orders
     * it with the network calls itself.
     */
    private suspend fun write(op: ForwardingOp, call: suspend (String) -> Forwarding): Unit = withContext(Dispatchers.Main.immediate) {
        var flowId = ""
        try {
            writes.withLock {
                flow("forwarding.${op.tag}") { id ->
                    flowId = id
                    val value = withContext(Dispatchers.IO) { call(id) }
                    writeSeq++
                    show(value, id, "action" to op.tag)
                }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("ui.forwarding", "${op.tag} failed: ${e::class.simpleName}", flowId)
            problem(e).let { failed(it.first, it.second) }
            load("after_${op.tag}")
        }
    }

    /** [op] as flow `forwarding.<tag>`, after any quiet check; a failure is a toast, and the status is read again. */
    private suspend fun run(op: ForwardingOp, call: suspend (String) -> Forwarding): Forwarding? = withContext(Dispatchers.Main.immediate) {
        phase = phase.start(op) ?: return@withContext null
        feedback = null
        val seq = writeSeq
        report()
        var flowId = ""
        var broke = false
        val value = try {
            calls.withLock {
                flow("forwarding.${op.tag}") { id ->
                    flowId = id
                    withContext(Dispatchers.IO) { call(id) }.also {
                        if (writeSeq == seq) show(it, id, "action" to op.tag)
                    }
                }
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            Log.warn("ui.forwarding", "${op.tag} failed: ${e::class.simpleName}", flowId)
            askedOtp(e)
            problem(e).let { failed(it.first, it.second) }
            broke = true
            null
        } finally {
            phase = phase.finish()
            report()
        }
        val wroteDuring = writeSeq != seq
        if (broke || wroteDuring) load("after_${op.tag}")
        value?.let { if (wroteDuring) status ?: it else it }
    }

    private fun show(value: Forwarding, flow: String, vararg extra: Pair<String, String>) {
        phase = phase.shown(value)
        val (state, fields) = when (value) {
            is Forwarding.Off -> "off" to mapOf()
            is Forwarding.Waiting -> "waiting" to mapOf("confirmation" to "${value.confirmation != null}", "last_mail_at" to "${value.lastMailAt}")
            is Forwarding.On -> "on" to mapOf("confirmation" to "${value.confirmation != null}", "last_mail_at" to "${value.lastMailAt}", "last_otp_at" to "${value.lastOtpAt}")
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
                "confirmation" to ((s as? Forwarding.Waiting)?.confirmation != null || (s as? Forwarding.On)?.confirmation != null),
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
