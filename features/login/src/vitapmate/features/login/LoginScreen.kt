package vitapmate.features.login

import vitapmate.ui.rememberHaptics
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.systemBarsPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.GraduationCap
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.User
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.launch
import vitapmate.core.CoreException
import vitapmate.core.CoreInterface
import vitapmate.core.SignIn
import vitapmate.data.AppCore
import vitapmate.data.Problem
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.PasswordField
import vitapmate.ui.ProblemBanner
import vitapmate.ui.Space

/**
 * Sign-in: VTOP username and password, then the emailed security OTP in a sheet when VTOP asks.
 * Wrong credentials show on the fields; offline, VTOP down and anything unexpected show in a
 * banner with "Try again" and "Details" (the flow in the Inspector). [onSignedIn] runs as soon
 * as VTOP lets the user in; the app then asks for the semester ([ChooseSemester]). [core] is the Rust
 * core (a stand-in in previews). Flows `login.sign_in`, `login.submit_otp`, `login.resend_otp`,
 * `login.cancel_otp`; log src `ui.login`.
 * TODO: server / UDP sign-in and email-OTP auto-fill go here (not in the prototype).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun LoginScreen(core: suspend () -> CoreInterface = { AppCore.await() }, onSignedIn: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val scope = rememberCoroutineScope()
    val focusManager = LocalFocusManager.current
    val passwordFocus = remember { FocusRequester() }
    var username by rememberSaveable { mutableStateOf("") }
    // Not saveable: the password never goes into the saved instance state.
    var password by remember { mutableStateOf("") }
    var usernameError by remember { mutableStateOf<String?>(null) }
    var passwordError by remember { mutableStateOf<String?>(null) }
    var rejected by remember { mutableStateOf(false) }
    var problem by remember { mutableStateOf<Problem?>(null) }
    var signingIn by remember { mutableStateOf(false) }
    var otp by remember { mutableStateOf<OtpChallenge?>(null) }
    val haptics = rememberHaptics()

    fun submit() {
        if (signingIn) return
        // A collapsed OTP prompt is still this sign-in: open it again instead of starting over.
        otp?.let {
            it.minimized = false
            return
        }
        usernameError = "Enter your VTOP username".takeIf { username.isBlank() }
        passwordError = "Enter your VTOP password".takeIf { password.isEmpty() }
        if (usernameError != null || passwordError != null) {
            Log.info("ui.login", "form incomplete", fields = mapOf("username" to "${username.isNotBlank()}", "password" to "${password.isNotEmpty()}"))
            return
        }
        focusManager.clearFocus()
        problem = null
        rejected = false
        scope.launch {
            signingIn = true
            var flowId = ""
            try {
                val result = flow("login.sign_in") { id ->
                    flowId = id
                    core().signIn(id, username.trim(), password)
                }
                when (result) {
                    is SignIn.Done -> onSignedIn()
                    is SignIn.NeedsOtp -> otp = OtpChallenge(result.message, "login", "ui.login")
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: CoreException.InvalidCredentials) {
                rejected = true
                passwordError = "VTOP didn't accept this username and password"
            } catch (e: CoreException.OtpRequired) {
                otp = OtpChallenge(e.reason, "login", "ui.login")
            } catch (e: Throwable) {
                problem = problemOf(e, flowId)
            } finally {
                signingIn = false
            }
            Log.info("ui.login", "sign-in result", flowId, mapOf("otp" to "${otp != null}", "rejected" to "$rejected", "problem" to "${problem?.kind}"))
        }
    }

    BoxWithConstraints(Modifier.fillMaxSize().background(scheme.background).imePadding().testTag("screen.login")) {
        Column(
            Modifier.verticalScroll(rememberScrollState()).fillMaxWidth().heightIn(min = maxHeight).systemBarsPadding()
                .padding(horizontal = Space.xl, vertical = Space.xxl),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Column(Modifier.widthIn(max = 420.dp).fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                Box(Modifier.size(88.dp).background(scheme.primaryContainer, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                    Icon(Lucide.GraduationCap, null, Modifier.size(40.dp), tint = scheme.onPrimaryContainer)
                }
                Spacer(Modifier.height(Space.xl))
                Text("Vitap Mate", style = MaterialTheme.typography.headlineLargeEmphasized)
                Spacer(Modifier.height(Space.sm))
                Text(
                    "Your timetable, attendance and marks from VTOP, on your phone. Sign in with your VTOP account.",
                    style = MaterialTheme.typography.bodyLarge,
                    color = scheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
                Spacer(Modifier.height(Space.xl))
                AnimatedVisibility(problem != null) {
                    problem?.let {
                        ProblemBanner(it, { problem = null }, Modifier.padding(bottom = Space.lg), onRetry = ::submit, onSignIn = null)
                    }
                }
                OutlinedTextField(
                    username,
                    {
                        username = it
                        usernameError = null
                        rejected = false
                    },
                    Modifier.fillMaxWidth().testTag("login.username").semantics { contentType = ContentType.Username },
                    enabled = !signingIn,
                    label = { Text("Username") },
                    leadingIcon = { Icon(Lucide.User, null) },
                    supportingText = usernameError?.let { { Text(it, Modifier.testTag("login.username.error")) } },
                    isError = usernameError != null || rejected,
                    keyboardOptions = KeyboardOptions(
                        capitalization = KeyboardCapitalization.None,
                        autoCorrectEnabled = false,
                        keyboardType = KeyboardType.Ascii,
                        imeAction = ImeAction.Next,
                    ),
                    keyboardActions = KeyboardActions(onNext = { passwordFocus.requestFocus() }),
                    singleLine = true,
                    shape = MaterialTheme.shapes.large,
                )
                Spacer(Modifier.height(Space.sm))
                PasswordField(
                    password,
                    {
                        password = it
                        passwordError = null
                        rejected = false
                    },
                    "Password",
                    "login.password",
                    Modifier.fillMaxWidth().focusRequester(passwordFocus),
                    enabled = !signingIn,
                    isError = passwordError != null,
                    leadingIcon = { Icon(Lucide.Lock, null) },
                    supportingText = passwordError?.let { { Text(it, Modifier.testTag("login.password.error")) } },
                    keyboardActions = KeyboardActions(onDone = { submit() }),
                    shape = MaterialTheme.shapes.large,
                )
                Spacer(Modifier.height(Space.xl))
                Button(
                    ::submit,
                    modifier = Modifier.fillMaxWidth().heightIn(min = ButtonDefaults.MediumContainerHeight).testTag("login.submit"),
                    enabled = !signingIn,
                    shapes = ButtonDefaults.shapes(),
                ) {
                    if (signingIn) {
                        LoadingIndicator(Modifier.size(24.dp).testTag("login.loading"))
                        Spacer(Modifier.size(ButtonDefaults.IconSpacing))
                        Text("Signing in…", style = MaterialTheme.typography.titleSmall)
                    } else {
                        Text("Sign in", style = MaterialTheme.typography.titleSmall)
                    }
                }
                Spacer(Modifier.height(Space.lg))
                Text(
                    "Trouble signing in? Check that you can sign in on the VTOP website, then try again here.",
                    style = MaterialTheme.typography.bodySmall,
                    color = scheme.onSurfaceVariant,
                    textAlign = TextAlign.Center,
                )
            }
        }
    }

    otp?.let { challenge ->
        OtpSheet(
            challenge,
            onSubmit = {
                scope.launch {
                    try {
                        if (challenge.submit(core())) {
                            haptics.confirm()
                            otp = null
                            onSignedIn()
                        } else if (challenge.error != null) {
                            haptics.reject()
                        }
                    } catch (e: CancellationException) {
                        throw e
                    } catch (e: Throwable) {
                        // The sign-in can't go on (session gone, VTOP changed): start again from the form.
                        otp = null
                        problem = problemOf(e, challenge.lastFlow)
                    }
                }
            },
            onResend = { scope.launch { challenge.resend(core()) } },
            onCancel = {
                otp = null
                scope.launch { flow("login.cancel_otp") { id -> Log.info("ui.login", "otp cancelled", id) } }
            },
        )
    }
}
