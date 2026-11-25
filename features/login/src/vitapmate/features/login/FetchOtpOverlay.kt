package vitapmate.features.login

import vitapmate.ui.rememberHaptics
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.launch
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.ProblemKind
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.logAction
import vitapmate.ui.ToastTone
import vitapmate.ui.showToast

/**
 * The OTP sheet for a sign-in that waits for the code ([OtpPrompt]); the app draws it over
 * everything. An accepted code closes it from Rust (the waiting fetches go on); Cancel or a
 * failure other than a wrong code or the network ends with [OtpPrompt.cancel]. The sheet keeps
 * its own UI state only ([OtpChallenge]: the code typed, minimised). Flows `fetch_otp.*`.
 */
@Composable
fun FetchOtpOverlay() {
    // Before the early return, so a flow launched while closing isn't cancelled with the sheet.
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    val shown by OtpPrompt.challenge.collectAsState()
    val open = shown ?: return
    val challenge = remember(open.id) { OtpChallenge(open.message, "fetch_otp", "ui.otp") }
    OtpSheet(
        challenge,
        onSubmit = {
            scope.launch {
                try {
                    if (challenge.submit(AppCore.await())) haptics.confirm() else if (challenge.error != null) haptics.reject()
                } catch (e: CancellationException) {
                    throw e
                } catch (e: Throwable) {
                    Log.warn("ui.otp", "otp failed: ${e::class.simpleName}", challenge.lastFlow)
                    OtpPrompt.cancel()
                    // Cancel on the cookie dialog was the student's choice: nothing to say.
                    problemOf(e, challenge.lastFlow).takeIf { it.kind != ProblemKind.ConsentDenied }
                        ?.let { showToast(it.title, it.message, tone = ToastTone.Error) }
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
