package vitapmate

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

/** [next]: every way from start to the shell and back to the login screen. */
class AppSessionTest {
    private fun run(vararg events: SessionEvent) = events.fold<SessionEvent, AppSession>(AppSession.Booting, ::next)

    private val out = SessionEvent.Account(null)
    private val u = SessionEvent.Account("u")

    @Test
    fun startSignedOutThenSignInPickAndOnboard() {
        assertEquals(AppSession.SignedOut, run(out))
        assertEquals(AppSession.LoadingAccount("22BCE1"), run(out, SessionEvent.Account("22BCE1")))
        val picking = run(out, SessionEvent.Account("22BCE1"), SessionEvent.AccountRead(picked = false, Setup.NotYet))
        assertEquals(AppSession.PickingSemester("22BCE1", firstTime = true), picking)
        val onboarding = next(picking, SessionEvent.PickChanged(picked = true, Setup.NotYet))
        assertEquals(AppSession.Onboarding("22BCE1"), onboarding)
        assertEquals(AppSession.Ready("22BCE1"), next(onboarding, SessionEvent.OnboardingDone))
    }

    @Test
    fun startSignedInWithAPickGoesStraightToTheShell() {
        assertEquals(AppSession.Ready("u"), run(u, SessionEvent.AccountRead(picked = true, Setup.NotYet)))
        // Onboarded before: the picker leads to the shell, not the setup page.
        val picking = run(u, SessionEvent.AccountRead(picked = false, Setup.Done))
        assertEquals(AppSession.Ready("u"), next(picking, SessionEvent.PickChanged(picked = true, Setup.Done)))
    }

    @Test
    fun aSetupPageLeftOpenOpensAgainAfterARestart() {
        assertEquals(AppSession.Onboarding("u"), run(u, SessionEvent.AccountRead(picked = true, Setup.Pending)))
        assertEquals(AppSession.PickingSemester("u", firstTime = true), run(u, SessionEvent.AccountRead(picked = false, Setup.Pending)))
    }

    @Test
    fun aForgottenPickOpensThePickerAgainAndSignOutEndsAnyPhase() {
        val ready = run(u, SessionEvent.AccountRead(picked = true, Setup.Done))
        assertEquals(AppSession.PickingSemester("u", firstTime = false), next(ready, SessionEvent.PickChanged(picked = false, Setup.Done)))
        for (s in listOf(AppSession.LoadingAccount("u"), AppSession.PickingSemester("u", true), AppSession.Onboarding("u"), ready)) {
            assertEquals(AppSession.SignedOut, next(s, out))
        }
    }

    @Test
    fun anotherStudentReadsTheirPickAgainAndTheSameOneStays() {
        val ready = run(u, SessionEvent.AccountRead(picked = true, Setup.Done))
        assertEquals(ready, next(ready, SessionEvent.Account("U")))
        assertEquals(AppSession.LoadingAccount("v"), next(ready, SessionEvent.Account("v")))
        val onboarding = AppSession.Onboarding("u")
        assertEquals(onboarding, next(onboarding, u))
    }

    @Test
    fun eventsThatDontApplyChangeNothing() {
        val failed = run(SessionEvent.BootError(IllegalStateException("no core")))
        assertIs<AppSession.BootFailed>(failed)
        assertEquals(failed, next(failed, u))
        assertEquals(AppSession.SignedOut, run(out, SessionEvent.AccountRead(picked = true, Setup.Done)))
        // The pick watch's first value, while the account is still read, waits for AccountRead.
        assertEquals(AppSession.LoadingAccount("u"), run(u, SessionEvent.PickChanged(picked = true, Setup.Done)))
        assertEquals(AppSession.Ready("u"), run(u, SessionEvent.AccountRead(true, Setup.Done), SessionEvent.BootError(IllegalStateException())))
    }
}
