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
import vitapmate.core.ForwardingState
import vitapmate.data.Gmail
import vitapmate.data.OtpSource
import vitapmate.features.login.MailForwarding
import vitapmate.features.login.OtpAutofill
import vitapmate.features.login.OtpAutofillSheet
import vitapmate.features.login.ago
import vitapmate.ui.AppTheme

/**
 * Settings → Account → OTP auto-fill: the one picked (mail forwarding or Gmail) and how it's doing,
 * a warning when Gmail's weekly grant ends soon; tapping opens [OtpAutofillSheet].
 */
@Composable
fun OtpAutofillSetting() {
    var open by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(Unit) { MailForwarding.load("settings") }
    val picked = OtpAutofill.selected()
    val forwarding = MailForwarding.status
    val account = Gmail.account
    var warn = false
    val hint = when (picked) {
        OtpSource.Forwarding -> when (forwarding?.state) {
            null -> "…"
            ForwardingState.OFF -> "Off · OTPs are typed by hand"
            ForwardingState.WAITING -> "Mail forwarding · finish setting up in Gmail"
            ForwardingState.ON -> "Mail forwarding" + (forwarding.lastOtpAt?.let { " · last OTP ${ago(it)}" } ?: "")
        }
        OtpSource.Gmail -> otpWarning()?.also { warn = true } ?: if (account == null) "Gmail · not connected yet" else "Gmail · ${account.email}"
    }
    GroupItem(
        "OTP auto-fill",
        "settings.otp_autofill",
        if (picked == OtpSource.Gmail) Lucide.Mail else Lucide.Forward,
        hint,
        onClick = { open = true },
        trailing = { Chevron() },
        tone = if (warn) AppTheme.tones.warning else null,
    )
    if (open) OtpAutofillSheet { open = false }
}

/** Gmail's grant ended or ends within 2 days (the row's warning, also in Settings' "Needs you"); null otherwise. */
fun otpWarning(): String? {
    if (OtpAutofill.selected() != OtpSource.Gmail) return null
    val account = Gmail.account ?: return null
    val days = account.daysLeft
    return when {
        account.needsReconnect -> "Gmail · reconnect needed"
        days != null && days <= 2 -> "Gmail · reconnect in ${if (days == 0L) "<1" else "$days"} d"
        else -> null
    }
}
