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
import vitapmate.data.OtpPrompt
import vitapmate.data.StoreCap
import vitapmate.data.VtopSync
import vitapmate.observability.flow

/**
 * More → Fetch all: every VTOP page fetched now, in the foreground ([VtopSync] with everything).
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
            // The page being fetched, from the activity entries VtopSync adds, or the OTP its sign-in waits for.
            val follow = launch {
                Activity.state.map { s -> s.running.lastOrNull { it.label.startsWith(LABEL) }?.label }.distinctUntilChanged().combine(OtpPrompt.challenge) { label, otp ->
                    if (otp != null) "Waiting for the OTP…" else label?.let { "Fetching ${it.substringAfter(": ").replace('_', ' ')}…" }
                }.collect { text -> text?.let { status = it } }
            }
            // The fetches wait for the code themselves; one that still wanted it waits again here
            // (the prompt stayed open past Rust's wait), or stops the run when it was closed.
            val result = VtopSync.run("sync.all", LABEL, everything = true, pageTimeoutMs = null) { _, f -> OtpPrompt.await(f) }
            follow.cancel()
            status = when (result.outcome) {
                VtopSync.Outcome.Done -> if (result.failed == 0) "Fetched ${result.fetched} pages" else "Fetched ${result.fetched} pages, ${result.failed} failed"
                VtopSync.Outcome.StoppedAtOtp -> "Stopped: the OTP wasn't entered"
                VtopSync.Outcome.Rejected -> "Stopped: VTOP turned down your password"
                VtopSync.Outcome.Failed -> "Couldn't start: try again"
            }
            running = false
            flow("store.after_fetch_all") { StoreCap.enforce(context, it) }
        }
    }

    private const val LABEL = "Fetch all"
}
