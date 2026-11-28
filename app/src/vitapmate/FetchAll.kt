package vitapmate

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import vitapmate.data.Activity
import vitapmate.data.AppScope
import vitapmate.data.attempt
import vitapmate.data.OtpPrompt
import vitapmate.core.SyncKind
import vitapmate.core.SyncOtp
import vitapmate.core.SyncOutcome
import vitapmate.data.AppCore
import vitapmate.data.StoreCap
import vitapmate.observability.flow

/**
 * More → Fetch all: every VTOP page fetched now, in the foreground (`Core.sync`, Fetch all).
 * When VTOP asks for the OTP, the usual prompt opens (auto-fill fills it when set up) and the
 * run goes on once the code is accepted, or stops if the prompt is dismissed. [status] is the
 * row's subtitle. Flow `sync.all`.
 */
object FetchAll {
    var running by mutableStateOf(false)
        private set

    /** What the row says: the page being fetched, or how the last run went. */
    var status by mutableStateOf<String?>(null)
        private set

    fun start(context: Context) {
        if (running) return
        running = true
        status = "Starting…"
        AppScope.launch {
            // The page being fetched, from the activity entries the sync adds, or the OTP its sign-in waits for.
            val follow = launch {
                Activity.state.map { s -> s.running.lastOrNull { it.label.startsWith(LABEL) }?.label }.distinctUntilChanged().combine(OtpPrompt.challenge) { label, otp ->
                    if (otp != null) "Waiting for the OTP…" else label?.let { "Fetching ${it.substringAfter(": ").replace('_', ' ')}…" }
                }.collect { text -> text?.let { status = it } }
            }
            // The fetches wait for the code themselves; one that still wanted it waits again here
            // (the prompt stayed open past Rust's wait), or stops the run when it was closed.
            val otp = object : SyncOtp {
                override suspend fun get(issuedAt: ULong?, flow: String): Boolean = OtpPrompt.await(flow)
            }
            val result = attempt { flow("sync.all") { f -> AppCore.await().sync(f, SyncKind.FETCH_ALL, null, otp) } }.getOrNull()
            follow.cancel()
            status = when (result?.outcome) {
                SyncOutcome.DONE -> if (result.failed == 0u) "Fetched ${result.fetched} pages" else "Fetched ${result.fetched} pages, ${result.failed} failed"
                SyncOutcome.STOPPED_AT_OTP -> "Stopped: the OTP wasn't entered"
                SyncOutcome.REJECTED -> "Stopped: VTOP turned down your password"
                SyncOutcome.FAILED, null -> "Couldn't start: try again"
                // Signed out meanwhile: nothing to say.
                SyncOutcome.CANCELLED -> null
            }
            running = false
            flow("store.after_fetch_all") { StoreCap.enforce(context, it) }
        }
    }

    private const val LABEL = "Fetch all"
}
