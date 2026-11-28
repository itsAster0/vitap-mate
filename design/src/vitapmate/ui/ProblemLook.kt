package vitapmate.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.vector.ImageVector
import com.composables.icons.lucide.CalendarRange
import com.composables.icons.lucide.CloudOff
import com.composables.icons.lucide.Cookie
import com.composables.icons.lucide.KeyRound
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MailCheck
import com.composables.icons.lucide.ServerCrash
import com.composables.icons.lucide.ShieldX
import com.composables.icons.lucide.TriangleAlert
import vitapmate.data.ProblemKind

/** The icon and tone a [ProblemKind] is drawn with, shared by [ProblemView] and [ProblemBanner]. */
@Composable
internal fun ProblemKind.look(): Pair<ImageVector, Tone> = AppTheme.tones.let { t ->
    icon to when (this) {
        ProblemKind.OFFLINE, ProblemKind.CONSENT_DENIED -> neutralTone
        ProblemKind.VTOP_DOWN -> t.warning
        ProblemKind.SIGN_IN_NEEDED, ProblemKind.OTP_NEEDED, ProblemKind.SEMESTER_NEEDED -> t.accentTone
        ProblemKind.OTP_REJECTED, ProblemKind.UNEXPECTED -> t.danger
    }
}

/** The icon a [ProblemKind] is drawn with (also on its toast). */
internal val ProblemKind.icon: ImageVector get() = when (this) {
    ProblemKind.OFFLINE -> Lucide.CloudOff
    ProblemKind.VTOP_DOWN -> Lucide.ServerCrash
    ProblemKind.SIGN_IN_NEEDED -> Lucide.KeyRound
    ProblemKind.OTP_NEEDED -> Lucide.MailCheck
    ProblemKind.OTP_REJECTED -> Lucide.ShieldX
    ProblemKind.SEMESTER_NEEDED -> Lucide.CalendarRange
    ProblemKind.CONSENT_DENIED -> Lucide.Cookie
    ProblemKind.UNEXPECTED -> Lucide.TriangleAlert
}

/** Composition local for the app's "go to sign-in" action, used by problem views for [ProblemKind.SIGN_IN_NEEDED]. */
val LocalSignIn = staticCompositionLocalOf<(() -> Unit)?> { null }
