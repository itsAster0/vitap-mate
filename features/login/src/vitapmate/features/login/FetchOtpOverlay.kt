package vitapmate.features.login

import vitapmate.ui.rememberHaptics
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.launch
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.logAction
import vitapmate.ui.ToastTone
import vitapmate.ui.showToast

/**
 * The OTP sheet for a fetch whose re-login wants the code ([OtpPrompt]); the app draws it over
 * everything. An accepted code ends with [OtpPrompt.done] (open pages fetch again); Cancel or a
 * failure other than a wrong code or the network ends with [OtpPrompt.cancel]. Flows `fetch_otp.*`.
 */
@Composable
fun FetchOtpOverlay() {
    // Before the early return, so a flow launched while closing isn't cancelled with the sheet.
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    val message = OtpPrompt.message ?: return
    val challenge = remember(message) { OtpChallenge(message, "fetch_otp", "ui.otp") }
    OtpSheet(
        challenge,
        onSubmit = {
            scope.launch {
                try {
                    if (challenge.submit(AppCore.await())) { haptics.confirm(); OtpPrompt.done() } else if (challenge.error != null) haptics.reject()
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Throwable) {
                    Log.warn("ui.otp", "otp failed: ${e::class.simpleName}", challenge.lastFlow)
                    OtpPrompt.cancel()
                    problemOf(e, challenge.lastFlow).let { showToast(it.title, it.message, tone = ToastTone.Error) }
                }
            }
        },
        onResend = { scope.launch { challenge.resend(AppCore.await()) } },
        onCancel = {
            OtpPrompt.cancel()
            scope.logAction("fetch_otp.cancel_otp", "ui.otp", "otp cancelled")
        },
    )
}
