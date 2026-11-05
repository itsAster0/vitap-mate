package vitapmate.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.vector.ImageVector
import com.composables.icons.lucide.CalendarRange
import com.composables.icons.lucide.CloudOff
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
        ProblemKind.Offline -> neutralTone
        ProblemKind.VtopDown -> t.warning
        ProblemKind.SignInNeeded, ProblemKind.OtpNeeded, ProblemKind.SemesterNeeded -> t.accentTone
        ProblemKind.OtpRejected, ProblemKind.Unexpected -> t.danger
    }
}

/** The icon a [ProblemKind] is drawn with (also on its toast). */
internal val ProblemKind.icon: ImageVector get() = when (this) {
    ProblemKind.Offline -> Lucide.CloudOff
    ProblemKind.VtopDown -> Lucide.ServerCrash
    ProblemKind.SignInNeeded -> Lucide.KeyRound
    ProblemKind.OtpNeeded -> Lucide.MailCheck
    ProblemKind.OtpRejected -> Lucide.ShieldX
    ProblemKind.SemesterNeeded -> Lucide.CalendarRange
    ProblemKind.Unexpected -> Lucide.TriangleAlert
}

/** Composition local for the app's "go to sign-in" action, used by problem views for [ProblemKind.SignInNeeded]. */
val LocalSignIn = staticCompositionLocalOf<(() -> Unit)?> { null }
