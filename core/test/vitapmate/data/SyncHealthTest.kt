package vitapmate.data

import kotlin.test.Test
import kotlin.test.assertEquals
import vitapmate.core.ForwardingOffKind

class SyncHealthTest {
    @Test
    fun readsSavedValuesOfEveryVersion() {
        assertEquals(SyncHealth.Active(0), SyncHealth.parse(null, 0))
        assertEquals(SyncHealth.Active(1), SyncHealth.parse(null, 1))
        assertEquals(SyncHealth.Active(0), SyncHealth.parse("", -3))
        assertEquals(SyncHealth.Paused(PauseReason.NoCode), SyncHealth.parse("no_code", 1))
        assertEquals(SyncHealth.Paused(PauseReason.GmailRefused()), SyncHealth.parse("gmail_refused", 0))
        assertEquals(SyncHealth.Paused(PauseReason.ForwardingOff()), SyncHealth.parse("forwarding_off", 0))
        // Older apps saved the text shown.
        assertEquals(SyncHealth.Paused(PauseReason.GmailReconnect), SyncHealth.parse("Gmail needs reconnecting", 0))
        assertEquals(SyncHealth.Paused(PauseReason.PasswordRejected), SyncHealth.parse("VTOP turned down your password", 0))
        assertEquals(SyncHealth.Paused(PauseReason.NotSetUp), SyncHealth.parse("OTP auto-fill isn't set up", 0))
        assertEquals(SyncHealth.Paused(PauseReason.GmailRefused("invalid_grant")), SyncHealth.parse("Gmail refused: invalid_grant", 0))
        assertEquals(
            SyncHealth.Paused(PauseReason.ForwardingOff(ForwardingOffKind.ADDRESS_DELETED)),
            SyncHealth.parse("Your forwarding address no longer exists. Set it up again.", 0),
        )
        assertEquals(
            SyncHealth.Paused(PauseReason.ForwardingOff(ForwardingOffKind.DEVICE_REMOVED)),
            SyncHealth.parse("This phone was removed from your account. Sign in again.", 0),
        )
        assertEquals(
            SyncHealth.Paused(PauseReason.ForwardingOff(ForwardingOffKind.NEEDS_SIGN_IN)),
            SyncHealth.parse("Mail forwarding reconnects once you're signed in.", 0),
        )
        assertEquals(SyncHealth.Paused(PauseReason.OtpFailed), SyncHealth.parse("something new", 0))
        val all = listOf(
            PauseReason.NotSetUp,
            PauseReason.GmailReconnect,
            PauseReason.GmailRefused(),
            PauseReason.GmailRefused("invalid_grant"),
            PauseReason.NoCode,
            PauseReason.CodeRefused,
            PauseReason.ForwardingOff(),
            PauseReason.ForwardingOff(ForwardingOffKind.NOT_SET_UP),
            PauseReason.ForwardingOff(ForwardingOffKind.NEEDS_SIGN_IN),
            PauseReason.ForwardingOff(ForwardingOffKind.ADDRESS_DELETED),
            PauseReason.ForwardingOff(ForwardingOffKind.DEVICE_REMOVED),
            PauseReason.PasswordRejected,
            PauseReason.OtpFailed,
        )
        all.forEach { assertEquals(it, PauseReason.parse(it.id)) }
    }

    @Test
    fun specificMessagesCarryTheirDetail() {
        assertEquals("Gmail refused: invalid_grant", PauseReason.GmailRefused("invalid_grant").text)
        assertEquals("Gmail refused the request", PauseReason.GmailRefused().text)
        assertEquals("This phone was removed from your account. Sign in again", PauseReason.ForwardingOff(ForwardingOffKind.DEVICE_REMOVED).text)
        assertEquals("Your forwarding address no longer exists. Set it up again", PauseReason.ForwardingOff(ForwardingOffKind.ADDRESS_DELETED).text)
        assertEquals("Mail forwarding reconnects once you're signed in", PauseReason.ForwardingOff(ForwardingOffKind.NEEDS_SIGN_IN).text)
        assertEquals("Mail forwarding is off", PauseReason.ForwardingOff(ForwardingOffKind.NOT_SET_UP).text)
    }

    @Test
    fun twoMissedCodesInARowPause() {
        val once = SyncHealth.OK.afterOtp(PauseReason.NoCode)
        assertEquals(SyncHealth.Active(1), once)
        assertEquals(SyncHealth.Paused(PauseReason.NoCode), once.afterOtp(PauseReason.NoCode))
        // A code that went through changes nothing until the run ends.
        assertEquals(once, once.afterOtp(null))
        assertEquals(SyncHealth.Paused(PauseReason.CodeRefused), once.afterOtp(PauseReason.CodeRefused))
        assertEquals(SyncHealth.Paused(PauseReason.NoCode), SyncHealth.Paused(PauseReason.GmailRefused("invalid_grant")).afterOtp(PauseReason.NoCode))
    }

    @Test
    fun resumesOnlyForItsReason() {
        val gmail = SyncHealth.Paused(PauseReason.GmailRefused("invalid_grant"))
        assertEquals(SyncHealth.OK, gmail.resumedIf(PauseReason::gmail))
        val password = SyncHealth.Paused(PauseReason.PasswordRejected)
        assertEquals(password, password.resumedIf(PauseReason::gmail))
        assertEquals(SyncHealth.Active(1), SyncHealth.Active(1).resumedIf { true })
    }
}
