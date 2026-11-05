package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import vitapmate.core.CoreException

class ProblemTest {
    private fun kind(e: Throwable) = problemOf(e, "f1").kind

    @Test
    fun everyCoreErrorHasAKind() {
        assertEquals(ProblemKind.Offline, kind(CoreException.Offline()))
        assertEquals(ProblemKind.VtopDown, kind(CoreException.VtopUnavailable("VTOP returned 502")))
        assertEquals(ProblemKind.SignInNeeded, kind(CoreException.InvalidCredentials()))
        assertEquals(ProblemKind.SignInNeeded, kind(CoreException.NotSignedIn()))
        assertEquals(ProblemKind.OtpNeeded, kind(CoreException.OtpRequired("otp sent")))
        assertEquals(ProblemKind.OtpRejected, kind(CoreException.OtpRejected("Invalid OTP")))
        assertEquals(ProblemKind.SemesterNeeded, kind(CoreException.SemesterNeeded()))
        assertEquals(ProblemKind.Unexpected, kind(CoreException.Other("parse error")))
        assertEquals(ProblemKind.Unexpected, kind(IllegalStateException("boom")))
    }

    @Test
    fun carriesTheFlowAndRetryable() {
        val p = problemOf(CoreException.Offline(), "abc123")
        assertEquals("abc123", p.flow)
        assertTrue(p.retryable)
        assertFalse(problemOf(CoreException.InvalidCredentials(), "x").retryable)
        assertFalse(problemOf(CoreException.NotSignedIn(), "x").retryable)
    }

    @Test
    fun vtopDownUsesTheReasonOrADefault() {
        assertEquals(
            "VTOP returned 502. This is on VTOP's side, not your connection; try again in a few minutes.",
            problemOf(CoreException.VtopUnavailable("VTOP returned 502."), "f").message,
        )
        assertTrue(problemOf(CoreException.VtopUnavailable("  "), "f").message.startsWith("VTOP is not answering."))
    }

    @Test
    fun otpRejectedShowsVtopTextOrADefault() {
        assertEquals("Invalid OTP", problemOf(CoreException.OtpRejected(" Invalid OTP "), "f").message)
        assertTrue(problemOf(CoreException.OtpRejected(""), "f").message.startsWith("That OTP didn't work"))
    }

    @Test
    fun onlyUserInitiatedProblemsToastUnlessBackgroundErrorsAreOn() {
        val offline = problemOf(CoreException.Offline(), "f")
        val otp = problemOf(CoreException.OtpRequired("otp sent"), "f")
        try {
            Settings.showBackgroundErrors = false
            assertTrue(offline.toasts)
            assertFalse(offline.copy(userInitiated = false).toasts)
            assertFalse(otp.toasts)
            Settings.showBackgroundErrors = true
            assertTrue(offline.copy(userInitiated = false).toasts)
            assertFalse(otp.copy(userInitiated = false).toasts)
        } finally {
            Settings.showBackgroundErrors = false
        }
    }

    @Test
    fun unexpectedNeverShowsInternals() {
        val p = problemOf(CoreException.Other("serde: missing field `x` at line 3"), "f")
        assertFalse("serde" in p.message)
    }
}
