package vitapmate.features.login

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.ShieldCheck
import vitapmate.ui.Space
import vitapmate.ui.nums

/** The collapsed OTP prompt: "OTP · 2:41", or "OTP expired · Resend"; tapping it reopens the sheet. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
internal fun OtpPill(challenge: OtpChallenge, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val scheme = MaterialTheme.colorScheme
    Surface(onClick, modifier.testTag("otp.reopen"), shape = CircleShape, color = scheme.surfaceContainerHigh, contentColor = scheme.primary, shadowElevation = 6.dp) {
        Row(Modifier.padding(horizontal = Space.lg, vertical = Space.md), verticalAlignment = Alignment.CenterVertically) {
            Icon(Lucide.ShieldCheck, null, Modifier.size(18.dp))
            Spacer(Modifier.width(Space.sm))
            Text(
                if (challenge.remaining > 0) "OTP · ${clock(challenge.remaining)}" else "OTP expired · Resend",
                style = MaterialTheme.typography.labelLargeEmphasized.nums(),
            )
        }
    }
}
