package vitapmate

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import com.composables.icons.lucide.Forward
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Mail
import vitapmate.core.Forwarding
import vitapmate.core.ForwardingHint
import vitapmate.data.Gmail
import vitapmate.data.OtpSource
import vitapmate.features.login.MailForwarding
import vitapmate.features.login.OtpAutofill
import vitapmate.features.login.OtpAutofillSheet
import vitapmate.features.login.ago
import vitapmate.ui.AppTheme

/**
 * Settings → Account → OTP auto-fill: the one picked (mail forwarding or Gmail) and how it's doing,
 * a warning when forwarding has a typed-OTP hint or Gmail's weekly grant ends soon; tapping opens
 * [OtpAutofillSheet].
 */
@Composable
fun OtpAutofillSetting() {
    var open by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(Unit) { MailForwarding.load("settings") }
    val picked = OtpAutofill.selected()
    val forwarding = MailForwarding.status
    val account = Gmail.account
    val warning = otpWarning()
    val hint = warning ?: when (picked) {
        OtpSource.Forwarding -> when (forwarding) {
            null -> "…"
            is Forwarding.Off -> "Mail forwarding · not set up yet, so OTPs are typed by hand"
            is Forwarding.Waiting -> "Mail forwarding · finish setting up in Gmail"
            is Forwarding.On -> "Mail forwarding" + (forwarding.lastOtpAt?.let { " · last OTP ${ago(it)}" } ?: "")
        }
        OtpSource.Gmail -> if (account == null) "Gmail · not connected yet" else "Gmail · ${account.email}"
    }
    GroupItem(
        "OTP auto-fill",
        "settings.otp_autofill",
        if (picked == OtpSource.Gmail) Lucide.Mail else Lucide.Forward,
        hint,
        onClick = { open = true },
        trailing = { Chevron() },
        tone = if (warning != null) AppTheme.tones.warning else null,
    )
    if (open) OtpAutofillSheet { open = false }
}

/**
 * Warning for the picked OTP auto-fill (the row's warning, also in Settings' "Needs you"):
 * forwarding's typed-OTP hint from Rust, or Gmail's grant ending within 2 days. Null otherwise.
 */
fun otpWarning(): String? = when (OtpAutofill.selected()) {
    OtpSource.Forwarding -> (MailForwarding.status as? Forwarding.On)?.let { on ->
        when (on.hint) {
            is ForwardingHint.TypedOtp -> "Mail forwarding · OTP typed by hand, check your filter"
            null -> if (on.confirmation != null) "Mail forwarding · new confirmation in Gmail" else null
        }
    }
    OtpSource.Gmail -> {
        val account = Gmail.account ?: return null
        val days = account.daysLeft
        when {
            account.needsReconnect -> "Gmail · reconnect needed"
            days != null && days <= 2 -> "Gmail · reconnect in ${if (days == 0L) "<1" else "$days"} d"
            else -> null
        }
    }
}
