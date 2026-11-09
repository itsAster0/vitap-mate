package vitapmate.features.login

import vitapmate.ui.OpenHaptic
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.material3.IconButton
import com.composables.icons.lucide.Minimize2
import vitapmate.observability.Log
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.offset
import androidx.compose.ui.unit.IntOffset
import vitapmate.ui.NavBarSpace
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Forward
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MailCheck
import kotlinx.coroutines.launch
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.Space
import vitapmate.ui.nums

/**
 * The OTP prompt as a modal bottom sheet: VTOP's message, six digit cells (a full code submits),
 * VTOP's reason when it turns a code down, the expiry countdown with Resend once it runs out,
 * Verify and "Cancel sign-in". Swiping it down, the scrim, back or the minimise button collapse it
 * into [OtpPill] (code, countdown and error kept), which reopens it; only "Cancel sign-in" runs
 * [onCancel]. It can't be collapsed while a code is being checked. With mail forwarding on, the
 * forwarded code fills in and submits by itself ([OtpChallenge.autofill]), shown as its source.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class, ExperimentalComposeUiApi::class)
@Composable
internal fun OtpSheet(challenge: OtpChallenge, onSubmit: () -> Unit, onResend: () -> Unit, onCancel: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val scope = rememberCoroutineScope()
    LaunchedEffect(challenge, challenge.round) { challenge.countDown() }
    val submit by rememberUpdatedState(onSubmit)
    LaunchedEffect(challenge, challenge.round) { challenge.autofill { if (challenge.canSubmit) submit() } }
    fun minimize(open: Boolean) {
        challenge.minimized = !open
        val what = if (open) "reopen_otp" else "minimize_otp"
        scope.logAction("${challenge.prefix}.$what", "ui.otp", what, mapOf("remaining_s" to "${challenge.remaining}"))
    }
    // Just above the floating nav bar, following it as it slides; at the bottom without one.
    val insets = WindowInsets.navigationBars
    AnimatedVisibility(
        challenge.minimized,
        Modifier.fillMaxSize().wrapContentSize(Alignment.BottomCenter).navigationBarsPadding().padding(bottom = Space.lg).offset {
            val rest = insets.getBottom(this) + Space.lg.roundToPx()
            IntOffset(0, -(NavBarSpace.px + Space.md.roundToPx() - rest).coerceAtLeast(0))
        },
        enter = scaleIn(MaterialTheme.motionScheme.fastSpatialSpec()) + fadeIn(),
        exit = scaleOut() + fadeOut(),
    ) { OtpPill(challenge, { minimize(open = true) }) }
    if (challenge.minimized) return
    val sheet = rememberBottomSheetState(
        initialValue = SheetValue.Hidden,
        enabledValues = setOf(SheetValue.Hidden, SheetValue.Expanded),
    ) { it != SheetValue.Hidden || !challenge.submitting }
    val focus = remember { FocusRequester() }
    LaunchedEffect(Unit) { focus.requestFocus() }
    OpenHaptic()
    ModalBottomSheet({ minimize(open = false) }, Modifier.semantics { testTagsAsResourceId = true }.testTag("otp.prompt"), sheet) {
        Box(Modifier.fillMaxWidth()) {
            IconButton(
                { scope.launch { sheet.hide() }.invokeOnCompletion { minimize(open = false) } },
                Modifier.align(Alignment.TopEnd).padding(end = Space.sm).testTag("otp.minimize"),
                enabled = !challenge.submitting,
            ) { Icon(Lucide.Minimize2, "Minimise") }
        }
        Column(
            Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).imePadding().padding(horizontal = Space.xl).padding(bottom = Space.xl),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Box(Modifier.size(72.dp).background(scheme.primaryContainer, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                Icon(Lucide.MailCheck, null, Modifier.size(32.dp), tint = scheme.onPrimaryContainer)
            }
            Spacer(Modifier.height(Space.lg))
            Text("Check your email", style = MaterialTheme.typography.headlineSmallEmphasized)
            Spacer(Modifier.height(Space.xs))
            Text(
                challenge.message,
                Modifier.testTag("otp.message"),
                style = MaterialTheme.typography.bodyMedium,
                color = scheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
            challenge.source?.let {
                Row(Modifier.padding(top = Space.sm).testTag("otp.source"), verticalAlignment = Alignment.CenterVertically) {
                    Icon(Lucide.Forward, null, Modifier.size(16.dp), tint = AppTheme.tones.success.base)
                    Spacer(Modifier.size(Space.xs))
                    Text(it, style = MaterialTheme.typography.labelLarge, color = scheme.onSurfaceVariant)
                }
            }
            Spacer(Modifier.height(Space.xl))
            OtpField(
                challenge.code,
                {
                    challenge.code = it
                    if (it.length == 6 && challenge.canSubmit) onSubmit()
                },
                isError = challenge.error != null,
                onDone = { if (challenge.canSubmit) onSubmit() },
                focus = focus,
            )
            challenge.error?.let {
                Text(
                    it,
                    Modifier.padding(top = Space.sm).testTag("otp.error"),
                    style = MaterialTheme.typography.bodyMedium,
                    color = scheme.error,
                    textAlign = TextAlign.Center,
                )
            }
            Spacer(Modifier.height(Space.md))
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    if (challenge.remaining > 0) "Expires in ${clock(challenge.remaining)}" else "Code expired",
                    Modifier.testTag("otp.countdown"),
                    style = MaterialTheme.typography.labelLarge.nums(),
                    color = scheme.onSurfaceVariant,
                )
                TextButton(onResend, Modifier.testTag("otp.resend"), enabled = challenge.canResend) {
                    Text(if (challenge.resending) "Sending…" else "Resend code")
                }
            }
            Spacer(Modifier.height(Space.md))
            Button(
                onSubmit,
                modifier = Modifier.fillMaxWidth().heightIn(min = ButtonDefaults.MediumContainerHeight).testTag("otp.submit"),
                enabled = challenge.canSubmit,
                shapes = ButtonDefaults.shapes(),
            ) {
                if (challenge.submitting) {
                    LoadingIndicator(Modifier.size(24.dp))
                    Spacer(Modifier.size(ButtonDefaults.IconSpacing))
                    Text("Verifying…")
                } else {
                    Text("Verify")
                }
            }
            TextButton(
                { scope.launch { sheet.hide() }.invokeOnCompletion { onCancel() } },
                Modifier.padding(top = Space.xs).testTag("otp.cancel"),
                enabled = !challenge.submitting,
            ) { Text("Cancel sign-in") }
        }
    }
}

internal fun clock(seconds: Int) = "%d:%02d".format(seconds / 60, seconds % 60)
