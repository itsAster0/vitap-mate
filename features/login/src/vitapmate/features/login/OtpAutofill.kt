package vitapmate.features.login

import vitapmate.data.Gmail
import vitapmate.data.OtpSource
import vitapmate.data.Settings

/**
 * One OTP auto-fill at a time: the student picks mail forwarding or Gmail ([Settings.otpSource])
 * in the OTP auto-fill sheet, and only that one fills in the code. The other keeps its setup
 * (switching back needs nothing redone) but is never read.
 */
object OtpAutofill {
    /** The pick; before there is one, whichever is set up (forwarding first). */
    fun selected(): OtpSource = Settings.otpSource ?: if (!MailForwarding.isOn && Gmail.account != null) OtpSource.Gmail else OtpSource.Forwarding

    fun ready(source: OtpSource) = when (source) {
        OtpSource.Forwarding -> MailForwarding.isOn
        OtpSource.Gmail -> Gmail.account?.needsReconnect == false
    }

    /** The one the OTP prompt reads, or null when the pick isn't set up (or needs reconnecting). [MailForwarding.load] first. */
    fun inUse(): OtpSource? = selected().takeIf(::ready)
}
