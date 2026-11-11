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

    /**
     * [data] (cached or fresh) from [updatedAt] (epoch seconds); [refreshing] while a fetch runs,
     * [waiting] as in [Loading] while it is queued; [problem] = the last refresh failed and [data]
     * was kept.
     */
    data class Ready<T>(val data: T, val updatedAt: Long, val refreshing: Boolean, val problem: Problem?, val waiting: String? = null) : PageState<T>

    /** Nothing cached and the fetch failed. */
    data class Failed(val problem: Problem) : PageState<Nothing>
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

private val NoToast = setOf(ProblemKind.SemesterNeeded, ProblemKind.OtpNeeded, ProblemKind.OtpRejected)

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

/** [SemesterNeeded]: no semester picked yet; the app shows the picker instead of a toast. */
enum class ProblemKind { Offline, VtopDown, SignInNeeded, OtpNeeded, OtpRejected, SemesterNeeded, Unexpected }

/** Opens the Inspector at a flow; the app provides it, any "Details" button calls it. */
fun interface InspectorOpener {
    fun open(flow: String)
}

val LocalInspector = staticCompositionLocalOf { InspectorOpener {} }

/** Opens the OTP prompt when [e] is a re-login asking for the code; true then (the prompt says it all, no toast). */
fun askedOtp(e: Throwable): Boolean {
    if (e !is CoreException.OtpRequired) return false
    OtpPrompt.ask(e.reason)
    return true
}

/** [block]'s value, or null when it fails; a cancellation still cancels (unlike `runCatching`). */
inline fun <T> orNull(block: () -> T): T? = try {
    block()
} catch (e: CancellationException) {
    throw e
} catch (_: Throwable) {
    null
}

/**
 * The one mapping from a failure to a [Problem] (texts from the Flutter app's `app_errors.dart`
 * and `common_toast.dart`). Anything that is not a [CoreException] is [ProblemKind.Unexpected].
 */
fun problemOf(e: Throwable, flow: String): Problem {
    fun p(kind: ProblemKind, title: String, message: String, retryable: Boolean = true) = Problem(kind, title, message, flow, retryable)
    return when (e) {
        is CoreException.Offline -> p(ProblemKind.Offline, "No Internet Connection", "We couldn't reach VTOP. Check your internet connection and try again.")
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
        is CoreException.Other -> p(ProblemKind.Unexpected, "VTOP Problem", UNEXPECTED)
        else -> p(ProblemKind.Unexpected, "Something Went Wrong", UNEXPECTED)
    }
}

private const val UNEXPECTED = "Something went wrong. Try again; if it keeps happening, open Details and share the log."
