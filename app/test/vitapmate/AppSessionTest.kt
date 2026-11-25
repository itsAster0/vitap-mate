package vitapmate

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

/** [next]: every way from start to the shell and back to the login screen. */
class AppSessionTest {
    private fun run(vararg events: SessionEvent) = events.fold<SessionEvent, AppSession>(AppSession.Booting, ::next)

    @Test
    fun startSignedOutThenSignInPickAndOnboard() {
        assertEquals(AppSession.SignedOut, run(SessionEvent.Booted(null)))
        assertEquals(AppSession.LoadingAccount(null), run(SessionEvent.Booted(null), SessionEvent.SignedIn))
        val picking = run(SessionEvent.Booted(null), SessionEvent.SignedIn, SessionEvent.UserRead("22BCE1"), SessionEvent.AccountRead(picked = false, onboarded = false))
        assertEquals(AppSession.PickingSemester("22BCE1", firstTime = true), picking)
        val onboarding = next(picking, SessionEvent.PickChanged(picked = true, onboarded = false))
        assertEquals(AppSession.Onboarding("22BCE1"), onboarding)
        assertEquals(AppSession.Ready("22BCE1"), next(onboarding, SessionEvent.OnboardingDone))
    }

    @Test
    fun startSignedInWithAPickGoesStraightToTheShell() {
        assertEquals(AppSession.Ready("u"), run(SessionEvent.Booted("u"), SessionEvent.AccountRead(picked = true, onboarded = false)))
        // Onboarded before: the picker leads to the shell, not the setup page.
        val picking = run(SessionEvent.Booted("u"), SessionEvent.AccountRead(picked = false, onboarded = true))
        assertEquals(AppSession.Ready("u"), next(picking, SessionEvent.PickChanged(picked = true, onboarded = true)))
    }

    @Test
    fun aForgottenPickOpensThePickerAgainAndSignOutEndsAnyPhase() {
        val ready = run(SessionEvent.Booted("u"), SessionEvent.AccountRead(picked = true, onboarded = true))
        assertEquals(AppSession.PickingSemester("u", firstTime = false), next(ready, SessionEvent.PickChanged(picked = false, onboarded = true)))
        for (s in listOf(AppSession.LoadingAccount("u"), AppSession.PickingSemester("u", true), AppSession.Onboarding("u"), ready)) {
            assertEquals(AppSession.SignedOut, next(s, SessionEvent.SignedOut))
        }
    }

    @Test
    fun eventsThatDontApplyChangeNothing() {
        val failed = run(SessionEvent.BootError(IllegalStateException("no core")))
        assertIs<AppSession.BootFailed>(failed)
        assertEquals(failed, next(failed, SessionEvent.SignedIn))
        assertEquals(AppSession.SignedOut, run(SessionEvent.Booted(null), SessionEvent.AccountRead(picked = true, onboarded = true)))
        // The pick watch's first value, while the account is still read, waits for AccountRead.
        assertEquals(AppSession.LoadingAccount("u"), run(SessionEvent.Booted("u"), SessionEvent.PickChanged(picked = true, onboarded = true)))
        assertEquals(AppSession.Ready("u"), run(SessionEvent.Booted("u"), SessionEvent.AccountRead(true, true), SessionEvent.Booted(null)))
    }
}
