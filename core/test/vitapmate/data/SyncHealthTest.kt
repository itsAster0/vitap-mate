package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals

class SyncHealthTest {
    @Test
    fun readsSavedValuesOfEveryVersion() {
        assertEquals(SyncHealth.Active(0), SyncHealth.parse(null, 0))
        assertEquals(SyncHealth.Active(1), SyncHealth.parse(null, 1))
        assertEquals(SyncHealth.Active(0), SyncHealth.parse("", -3))
        assertEquals(SyncHealth.Paused(PauseReason.NoCode), SyncHealth.parse("no_code", 1))
        // Older apps saved the text shown.
        assertEquals(SyncHealth.Paused(PauseReason.GmailReconnect), SyncHealth.parse("Gmail needs reconnecting", 0))
        assertEquals(SyncHealth.Paused(PauseReason.PasswordRejected), SyncHealth.parse("VTOP turned down your password", 0))
        assertEquals(SyncHealth.Paused(PauseReason.NotSetUp), SyncHealth.parse("OTP auto-fill isn't set up", 0))
        assertEquals(SyncHealth.Paused(PauseReason.GmailRefused), SyncHealth.parse("Gmail refused: invalid_grant", 0))
        assertEquals(SyncHealth.Paused(PauseReason.ForwardingOff), SyncHealth.parse("Your forwarding address no longer exists. Set it up again.", 0))
        assertEquals(SyncHealth.Paused(PauseReason.OtpFailed), SyncHealth.parse("something new", 0))
        PauseReason.entries.forEach { assertEquals(it, PauseReason.parse(it.id)) }
    }

    @Test
    fun twoMissedCodesInARowPause() {
        val once = SyncHealth.OK.afterOtp(PauseReason.NoCode.text)
        assertEquals(SyncHealth.Active(1), once)
        assertEquals(SyncHealth.Paused(PauseReason.NoCode), once.afterOtp(PauseReason.NoCode.text))
        // A code that went through changes nothing until the run ends.
        assertEquals(once, once.afterOtp(null))
        assertEquals(SyncHealth.Paused(PauseReason.CodeRefused), once.afterOtp("VTOP didn't take the code"))
        assertEquals(SyncHealth.Paused(PauseReason.NoCode), SyncHealth.Paused(PauseReason.GmailRefused).afterOtp(PauseReason.NoCode.text))
    }

    @Test
    fun resumesOnlyForItsReason() {
        val gmail = SyncHealth.Paused(PauseReason.GmailRefused)
        assertEquals(SyncHealth.OK, gmail.resumedIf(PauseReason::gmail))
        val password = SyncHealth.Paused(PauseReason.PasswordRejected)
        assertEquals(password, password.resumedIf(PauseReason::gmail))
        assertEquals(SyncHealth.Active(1), SyncHealth.Active(1).resumedIf { true })
    }
}
