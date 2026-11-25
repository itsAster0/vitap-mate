package vitapmate.data

import androidx.compose.runtime.staticCompositionLocalOf
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.flow.MutableSharedFlow
import vitapmate.core.CoreException

/** Every screen's data (from [rememberPage]), as design's `PageScaffold` draws it. */
sealed interface PageState<out T> {
    /**
     * Nothing to show yet (no cache; the first fetch runs). [waiting]: the fetch is queued behind
     * other pages ("Waiting for Attendance to finish"), set after a moment ([PageFetches]).
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

    /** [text] as [Refreshing.waiting]; only a running fetch waits. */
    fun waiting(text: String?): PageActivity = if (this is Refreshing) Refreshing(text) else this
}

/**
 * What went wrong, in words for the student: [title], [message] (says what to do), and the [flow]
 * it happened in. [userInitiated]: the student asked for the work (a pull, a button, a retry, a
 * submit); false for automatic work (auto refresh, the sweep), which toasts only with
 * [Settings.showBackgroundErrors] ([toasts]).
 */
data class Problem(val kind: ProblemKind, val title: String, val message: String, val flow: String, val retryable: Boolean, val userInitiated: Boolean = true)

/** Whether [this] shows as a toast: never for an OTP or semester prompt (they have their own UI). */
val Problem.toasts: Boolean get() = kind !in NoToast && (userInitiated || Settings.showBackgroundErrors)

private val NoToast = setOf(ProblemKind.SemesterNeeded, ProblemKind.OtpNeeded, ProblemKind.OtpRejected, ProblemKind.ConsentDenied)

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

/**
 * [SemesterNeeded]: no semester picked yet; the app shows the picker instead of a toast.
 * [ConsentDenied]: the student cancelled sending their VTOP cookie; no toast, only the problem view.
 */
enum class ProblemKind { Offline, VtopDown, SignInNeeded, OtpNeeded, OtpRejected, SemesterNeeded, ConsentDenied, Unexpected }

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
 * The one mapping from a failure to a [Problem] (texts from the Flutter app's `app_errors.dart`
 * and `common_toast.dart`). Anything that is not a [CoreException] is [ProblemKind.Unexpected].
 * [server]: the call went to our server, not VTOP, so its failures don't blame VTOP.
 */
fun problemOf(e: Throwable, flow: String, server: Boolean = false): Problem {
    fun p(kind: ProblemKind, title: String, message: String, retryable: Boolean = true) = Problem(kind, title, message, flow, retryable)
    return when (e) {
        is CoreException.Offline -> p(ProblemKind.Offline, "No Internet Connection", "We couldn't reach ${if (server) "the server" else "VTOP"}. Check your internet connection and try again.")
        is CoreException.VtopUnavailable -> p(
            ProblemKind.VtopDown,
            "VTOP Is Down",
            "${e.reason.trim().trimEnd('.').ifEmpty { "VTOP is not answering" }}. This is on VTOP's side, not your connection; try again in a few minutes.",
        )
        is CoreException.InvalidCredentials -> {
            AccountState.rejected = true
            p(
                ProblemKind.SignInNeeded,
                "VTOP Sign-In Failed",
                "VTOP rejected the saved password. Update it in Settings, under Username and password.",
                retryable = false,
            )
        }
        is CoreException.NotSignedIn -> p(ProblemKind.SignInNeeded, "Sign-In Needed", "Your saved session expired. Please sign in again.", retryable = false)
        is CoreException.OtpRequired -> p(ProblemKind.OtpNeeded, "Verify Your Sign-In", "We need to verify this sign-in with the OTP sent to your registered email.")
        is CoreException.OtpRejected -> p(
            ProblemKind.OtpRejected,
            "OTP Not Accepted",
            e.reason.trim().ifEmpty { "That OTP didn't work. Check the code, or ask for a new one, and try again." },
        )
        is CoreException.SemesterNeeded -> p(
            ProblemKind.SemesterNeeded,
            "Choose Your Semester",
            "Pick the semester to show in Settings → Account → Semester.",
            retryable = false,
        )
        is CoreException.ConsentDenied -> p(ProblemKind.ConsentDenied, "Not Sent", "Not sent: your VTOP cookie stays on the phone.")
        // A "Not now" on the registration number's sheet: as quiet as the cookie's no.
        is CoreException.RegnoNotShared -> p(ProblemKind.ConsentDenied, "Not Shared", "This needs your registration number on the vitapmate server. Share it from Settings → Connections.")
        is CoreException.Other -> if (server) p(ProblemKind.Unexpected, "Server Problem", e.reason.trim().ifEmpty { UNEXPECTED }) else p(ProblemKind.Unexpected, "VTOP Problem", UNEXPECTED)
        else -> p(ProblemKind.Unexpected, "Something Went Wrong", UNEXPECTED)
    }
}

private const val UNEXPECTED = "Something went wrong. Try again; if it keeps happening, open Details and share the log."
