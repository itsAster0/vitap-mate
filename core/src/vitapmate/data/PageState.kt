package vitapmate.data

import androidx.compose.runtime.staticCompositionLocalOf
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import vitapmate.core.CoreException
import vitapmate.core.problemToasts
import vitapmate.core.problemOf as rustProblemOf

/** Every screen's data (from [rememberPage]), as design's `PageScaffold` draws it. */
sealed interface PageState<out T> {
    /**
     * Nothing to show yet (no cache; the first fetch runs). [waiting]: the fetch is queued behind
     * other pages ("Waiting for Attendance to finish"), set after a moment (Rust's page watch).
     */
    data class Loading(val waiting: String? = null) : PageState<Nothing>

    /** [data] (cached or fresh) from [updatedAt] (epoch seconds), and what its refresh is doing. */
    data class Ready<T>(val data: T, val updatedAt: Long, val activity: PageActivity = PageActivity.Idle) : PageState<T>

    /** Nothing cached and the fetch failed. */
    data class Failed(val problem: Problem) : PageState<Nothing>
}

/** What a [PageState.Ready] page's refresh is doing. (Not `Activity`: that is the run tracker.) */
sealed interface PageActivity {
    data object Idle : PageActivity

    /** A fetch runs; [waiting] as in [PageState.Loading] while it is queued. */
    data class Refreshing(val waiting: String? = null) : PageActivity

    /** The last refresh failed with [problem]; the data was kept. */
    data class Stale(val problem: Problem) : PageActivity

    /** After a fetch that ended without an answer (cancelled, or dropped from the queue). */
    fun settled(): PageActivity = if (this is Refreshing) Idle else this
}

/**
 * What went wrong, in words for the student (Rust's `Problem`): title, message (says what to do),
 * and the flow it happened in. `userInitiated`: the student asked for the work (a pull, a button,
 * a retry, a submit); false for automatic work (auto refresh, the sweep), which toasts only with
 * [Settings.showBackgroundErrors] ([toasts]).
 */
typealias Problem = vitapmate.core.Problem

/**
 * SEMESTER_NEEDED: no semester picked yet; the app shows the picker instead of a toast.
 * CONSENT_DENIED: the student cancelled sending their VTOP cookie; no toast, only the problem view.
 */
typealias ProblemKind = vitapmate.core.ProblemKind

/** Whether [this] shows as a toast: never for an OTP or semester prompt (they have their own UI). */
val Problem.toasts: Boolean get() = problemToasts(this, Settings.showBackgroundErrors)

/**
 * Failures of automatic work that has no screen of its own (calendar and exam-day refreshes, the
 * sweep): [report] passes them on as [toasts] allow; the app shows each as a toast.
 */
object BackgroundProblems {
    val shown = MutableSharedFlow<Problem>(extraBufferCapacity = 1, onBufferOverflow = BufferOverflow.DROP_OLDEST)

    fun report(e: Throwable, flow: String) {
        problemOf(e, flow).copy(userInitiated = false).takeIf { it.toasts }?.let { shown.tryEmit(it) }
    }
}

/** Opens the Inspector at a flow; the app provides it, any "Details" button calls it. */
fun interface InspectorOpener {
    fun open(flow: String)
}

val LocalInspector = staticCompositionLocalOf { InspectorOpener {} }

/**
 * Whether [e] is a sign-in that wanted the OTP: Rust opened the prompt for it and waited, or
 * nobody watched the call ([OtpPrompt]); true then, and no toast (the prompt said it all).
 */
fun askedOtp(e: Throwable): Boolean = e is CoreException.OtpRequired

/**
 * [block]'s value or failure, like `runCatching`, but a cancellation still cancels: a
 * cancelled coroutine has to stop, not count as a failed call. Inline, so [block] may suspend
 * and return from its caller.
 */
inline fun <T> attempt(block: () -> T): Result<T> = try {
    Result.success(block())
} catch (e: CancellationException) {
    throw e
} catch (e: Throwable) {
    Result.failure(e)
}

/** [block]'s value, or null when it fails; a cancellation still cancels (unlike `runCatching`). */
inline fun <T> orNull(block: () -> T): T? = attempt(block).getOrNull()

/**
 * [cause] of a call to our server in [flow], as the Server and Connected apps screens get it
 * back, so their problem view's Details opens that flow.
 */
class ServerFailure(val flow: String, override val cause: Throwable) : Exception(cause) {
    val problem get() = problemOf(cause, flow, server = true)
}

/**
 * A failure as a [Problem]: Rust maps a [CoreException] (its texts are the Flutter app's
 * `app_errors.dart` and `common_toast.dart`); anything else is [ProblemKind.UNEXPECTED].
 * [server]: the call went to our server, not VTOP, so its failures don't blame VTOP.
 */
fun problemOf(e: Throwable, flow: String, server: Boolean = false): Problem =
    (e as? CoreException)?.let { rustProblemOf(it, flow, server) }
        ?: Problem(ProblemKind.UNEXPECTED, "Something Went Wrong", UNEXPECTED, flow, retryable = true)

private const val UNEXPECTED = "Something went wrong. Try again; if it keeps happening, open Details and share the log."
