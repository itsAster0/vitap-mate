package vitapmate.data

/** Why background sync is paused: [text] is shown ("Paused: …"), [id] is saved. */
enum class PauseReason(val id: String, val text: String) {
    NotSetUp("not_set_up", "OTP auto-fill isn't set up"),
    GmailReconnect("gmail_reconnect", "Gmail needs reconnecting"),
    GmailRefused("gmail_refused", "Gmail refused the request"),
    NoCode("no_code", "No code came in time"),
    CodeRefused("code_refused", "VTOP didn't take the code"),
    ForwardingOff("forwarding_off", "Mail forwarding is off"),
    PasswordRejected("password_rejected", "VTOP turned down your password"),
    OtpFailed("otp_failed", "The OTP couldn't be filled in"),
    ;

    /** Connecting Gmail again clears it. */
    val gmail get() = this == GmailReconnect || this == GmailRefused

    companion object {
        /** A saved [id], the text an older app saved, or why the OTP auto-fill failed (its own words). */
        fun parse(saved: String): PauseReason = entries.firstOrNull { it.id == saved || it.text == saved } ?: when {
            saved.startsWith("Gmail refused") -> GmailRefused
            // Rust's ForwardingOff reasons.
            saved.contains("forwarding", ignoreCase = true) || saved.contains("removed from your account") -> ForwardingOff
            else -> OtpFailed
        }
    }
}

/**
 * How background sync stands ([Settings.syncHealth]): [Active] counts the runs in a row whose OTP
 * code never came; [Paused] says why, until a run goes through (runs still try while paused).
 */
sealed interface SyncHealth {
    data class Active(val otpMisses: Int = 0) : SyncHealth

    data class Paused(val reason: PauseReason) : SyncHealth

    val pausedFor: PauseReason? get() = (this as? Paused)?.reason

    /**
     * After a run's OTP auto-fill: [why] is its failure in its own words, null when the code went
     * through (which says nothing until the run ends). VTOP mails late at times, so one code that
     * never came is only that run's luck; [MISSES_TO_PAUSE] in a row pause.
     */
    fun afterOtp(why: String?): SyncHealth {
        val reason = why?.let(PauseReason::parse) ?: return this
        if (reason != PauseReason.NoCode) return Paused(reason)
        return when (this) {
            is Active -> if (otpMisses + 1 >= MISSES_TO_PAUSE) Paused(PauseReason.NoCode) else Active(otpMisses + 1)
            is Paused -> Paused(PauseReason.NoCode)
        }
    }

    /** Active again when paused for a reason [cleared] covers (the student fixed it). */
    fun resumedIf(cleared: (PauseReason) -> Boolean): SyncHealth = if (this is Paused && cleared(reason)) OK else this

    companion object {
        val OK: SyncHealth = Active(0)
        const val MISSES_TO_PAUSE = 2

        /** From the saved values: [paused] (null or empty when not), [misses] the count. */
        fun parse(paused: String?, misses: Int): SyncHealth =
            paused?.takeIf { it.isNotEmpty() }?.let { Paused(PauseReason.parse(it)) } ?: Active(misses.coerceAtLeast(0))
    }
}
