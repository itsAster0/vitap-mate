package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import vitapmate.core.CoreException
import vitapmate.core.OtpRejection

class ProblemTest {
    private fun kind(e: Throwable) = problemOf(e, "f1").kind

    @Test
    fun everyCoreErrorHasAKind() {
        assertEquals(ProblemKind.Offline, kind(CoreException.Offline()))
        assertEquals(ProblemKind.VtopDown, kind(CoreException.VtopUnavailable("VTOP returned 502")))
        assertEquals(ProblemKind.SignInNeeded, kind(CoreException.InvalidCredentials()))
        assertEquals(ProblemKind.SignInNeeded, kind(CoreException.NotSignedIn()))
        assertEquals(ProblemKind.OtpNeeded, kind(CoreException.OtpRequired("otp sent", null)))
        assertEquals(ProblemKind.OtpRejected, kind(CoreException.OtpRejected(OtpRejection.WRONG, "Invalid OTP")))
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
        assertEquals("Invalid OTP", problemOf(CoreException.OtpRejected(OtpRejection.WRONG, " Invalid OTP "), "f").message)
        assertTrue(problemOf(CoreException.OtpRejected(OtpRejection.WRONG, ""), "f").message.startsWith("That OTP didn't work"))
    }

    @Test
    fun onlyUserInitiatedProblemsToastUnlessBackgroundErrorsAreOn() {
        val offline = problemOf(CoreException.Offline(), "f")
        val otp = problemOf(CoreException.OtpRequired("otp sent", null), "f")
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

    @Test
    fun serverFailuresBlameTheServerAndKeepTheirFlow() {
        val offline = ServerFailure("s1", CoreException.Offline()).problem
        assertEquals("s1", offline.flow)
        assertTrue("the server" in offline.message && "VTOP" !in offline.message)
        assertEquals("Proof rejected", ServerFailure("s2", CoreException.Other(" Proof rejected ")).problem.message)
        assertTrue(ServerFailure("s3", CoreException.Other("")).problem.message.startsWith("Something went wrong"))
    }
}
