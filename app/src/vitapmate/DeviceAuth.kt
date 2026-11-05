package vitapmate

import android.app.Activity
import android.app.KeyguardManager
import android.content.Context
import android.hardware.biometrics.BiometricManager.Authenticators
import android.hardware.biometrics.BiometricPrompt
import android.os.Build
import android.os.CancellationSignal
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.platform.LocalContext
import vitapmate.observability.Log

/** Asks for the phone's lock (biometric or PIN/pattern/password) before [onPassed]; cancelling runs nothing. */
fun interface DeviceAuth {
    fun confirm(title: String, onPassed: () -> Unit)
}

/**
 * The platform's [DeviceAuth]: a phone without a lock passes straight through; API 30+ and 28–29 use
 * [BiometricPrompt] with the screen lock allowed, older ones the keyguard's confirm screen.
 */
@Composable
fun rememberDeviceAuth(): DeviceAuth {
    val context = LocalContext.current
    var pending by remember { mutableStateOf<(() -> Unit)?>(null) }
    val confirmScreen = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) {
        val passed = it.resultCode == Activity.RESULT_OK
        Log.info("ui.device_auth", "keyguard result", fields = mapOf("passed" to "$passed"))
        if (passed) pending?.invoke()
        pending = null
    }
    return remember(context, confirmScreen) {
        DeviceAuth { title, onPassed ->
            val keyguard = context.getSystemService(KeyguardManager::class.java)
            when {
                keyguard?.isDeviceSecure != true -> onPassed()
                Build.VERSION.SDK_INT >= Build.VERSION_CODES.P -> prompt(context, title, onPassed)
                else -> {
                    @Suppress("DEPRECATION")
                    val intent = keyguard.createConfirmDeviceCredentialIntent(title, null)
                    if (intent == null) {
                        onPassed()
                    } else {
                        pending = onPassed
                        confirmScreen.launch(intent)
                    }
                }
            }
        }
    }
}

@androidx.annotation.RequiresApi(Build.VERSION_CODES.P)
private fun prompt(context: Context, title: String, onPassed: () -> Unit) {
    val builder = BiometricPrompt.Builder(context).setTitle(title)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
        builder.setAllowedAuthenticators(Authenticators.BIOMETRIC_STRONG or Authenticators.BIOMETRIC_WEAK or Authenticators.DEVICE_CREDENTIAL)
    } else {
        @Suppress("DEPRECATION")
        builder.setDeviceCredentialAllowed(true)
    }
    builder.build().authenticate(
        CancellationSignal(),
        context.mainExecutor,
        object : BiometricPrompt.AuthenticationCallback() {
            override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                Log.info("ui.device_auth", "passed")
                onPassed()
            }

            override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                Log.info("ui.device_auth", "not passed", fields = mapOf("code" to "$errorCode"))
            }
        },
    )
}
