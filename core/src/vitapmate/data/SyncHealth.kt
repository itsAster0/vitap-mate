package vitapmate.data

import vitapmate.core.ForwardingOffKind

/** Why background sync is paused: [text] is shown ("Paused: …"), [id] is saved. */
sealed interface PauseReason {
    val id: String
    val text: String

    /** Connecting Gmail again clears it. */
    val gmail: Boolean get() = this is GmailReconnect || this is GmailRefused

    data object NotSetUp : PauseReason {
        override val id = "not_set_up"
        override val text = "OTP auto-fill isn't set up"
    }

    data object GmailReconnect : PauseReason {
        override val id = "gmail_reconnect"
        override val text = "Gmail needs reconnecting"
    }

    data class GmailRefused(val code: String? = null) : PauseReason {
        private val clean get() = code?.trim()?.takeIf { it.isNotEmpty() }
        override val id get() = clean?.let { "$ID:$it" } ?: ID
        override val text get() = clean?.let { "Gmail refused: $it" } ?: "Gmail refused the request"

        companion object {
            const val ID = "gmail_refused"
        }
    }

    data object NoCode : PauseReason {
        override val id = "no_code"
        override val text = "No code came in time"
    }

    data object CodeRefused : PauseReason {
        override val id = "code_refused"
        override val text = "VTOP didn't take the code"
    }

    data class ForwardingOff(val kind: ForwardingOffKind? = null) : PauseReason {
        override val id
            get() = when (kind) {
                null -> ID
                ForwardingOffKind.NOT_SET_UP -> "$ID:not_set_up"
                ForwardingOffKind.NEEDS_SIGN_IN -> "$ID:needs_sign_in"
                ForwardingOffKind.ADDRESS_DELETED -> "$ID:address_deleted"
                ForwardingOffKind.DEVICE_REMOVED -> "$ID:device_removed"
            }
        override val text
            get() = when (kind) {
                null, ForwardingOffKind.NOT_SET_UP -> "Mail forwarding is off"
                ForwardingOffKind.NEEDS_SIGN_IN -> "Mail forwarding reconnects once you're signed in"
                ForwardingOffKind.ADDRESS_DELETED -> "Your forwarding address no longer exists. Set it up again"
                ForwardingOffKind.DEVICE_REMOVED -> "This phone was removed from your account. Sign in again"
            }

        companion object {
            const val ID = "forwarding_off"

            internal fun parseKind(raw: String): ForwardingOffKind? = when (raw) {
                "not_set_up" -> ForwardingOffKind.NOT_SET_UP
                "needs_sign_in" -> ForwardingOffKind.NEEDS_SIGN_IN
                "address_deleted" -> ForwardingOffKind.ADDRESS_DELETED
                "device_removed" -> ForwardingOffKind.DEVICE_REMOVED
                else -> null
            }
        }
    }

    data object PasswordRejected : PauseReason {
        override val id = "password_rejected"
        override val text = "VTOP turned down your password"
    }

    data object OtpFailed : PauseReason {
        override val id = "otp_failed"
        override val text = "The OTP couldn't be filled in"
    }

    companion object {
        // Lazy: nested data objects initialize after the enclosing interface's companion.
        private val simple by lazy { listOf(NotSetUp, GmailReconnect, NoCode, CodeRefused, PasswordRejected, OtpFailed) }

        /** Reads a saved [id], or migrates the display text an older app saved. */
        fun parse(saved: String): PauseReason {
            simple.firstOrNull { it.id == saved || it.text == saved }?.let { return it }
            if (saved == GmailRefused.ID || saved == "Gmail refused the request") return GmailRefused()
            if (saved.startsWith("${GmailRefused.ID}:")) return GmailRefused(saved.substringAfter(':'))
            if (saved == ForwardingOff.ID || saved == "Mail forwarding is off") return ForwardingOff()
            if (saved.startsWith("${ForwardingOff.ID}:")) {
                return ForwardingOff(ForwardingOff.parseKind(saved.substringAfter(':')))
            }
            // One-time migration for older apps that persisted free-text failure messages.
            return when {
                saved.startsWith("Gmail refused:") -> GmailRefused(saved.substringAfter(':').trim())
                saved.startsWith("Gmail refused") -> GmailRefused()
                saved.contains("removed from your account", ignoreCase = true) -> ForwardingOff(ForwardingOffKind.DEVICE_REMOVED)
                saved.contains("no longer exists", ignoreCase = true) -> ForwardingOff(ForwardingOffKind.ADDRESS_DELETED)
                saved.contains("reconnects once", ignoreCase = true) -> ForwardingOff(ForwardingOffKind.NEEDS_SIGN_IN)
                saved.contains("forwarding", ignoreCase = true) -> ForwardingOff(ForwardingOffKind.NOT_SET_UP)
                else -> OtpFailed
            }
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
     * After a run's OTP auto-fill: [why] is its typed failure, null when the code went through
     * (which says nothing until the run ends). VTOP mails late at times, so one code that never
     * came is only that run's luck; [MISSES_TO_PAUSE] in a row pause.
     */
    fun afterOtp(why: PauseReason?): SyncHealth {
        val reason = why ?: return this
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
