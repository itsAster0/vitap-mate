package vitapmate.features.login

import vitapmate.ui.rememberHaptics
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import com.composables.icons.lucide.CalendarDays
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Info
import vitapmate.ui.AppTheme
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
import androidx.compose.runtime.LaunchedEffect
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
import androidx.compose.ui.platform.LocalContext
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
import vitapmate.data.SavedPasswords
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.PasswordField
import vitapmate.ui.ProblemBanner
import vitapmate.ui.Space

/**
 * Sign-in: VTOP username and password, then the emailed security OTP in a sheet when VTOP asks.
 * Wrong credentials show on the fields; offline, VTOP down and anything unexpected show in a
 * banner with "Try again" and "Details" (the flow in the Inspector). [onSignedIn] runs as soon
 * as VTOP lets the user in; the app then asks for the semester ([ChooseSemester]). [core] is the Rust
 * core (a stand-in in previews). Flows `login.sign_in`, `login.submit_otp`, `login.resend_otp`,
 * `login.cancel_otp`; log src `ui.login`. Fills from and offers to save to the phone's password
 * manager ([SavedPasswords]).
 * TODO: server / UDP sign-in and email-OTP auto-fill go here (not in the prototype).
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun LoginScreen(core: suspend () -> CoreInterface = { AppCore.await() }, onSignedIn: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val cardColor = AppTheme.colors.card
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
    val context = LocalContext.current
    // A saved password from the phone's password manager, offered once when the form opens empty.
    var picked by remember { mutableStateOf<Pair<String, String>?>(null) }
    var offered by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(Unit) {
        if (offered || username.isNotEmpty()) return@LaunchedEffect
        offered = true
        SavedPasswords.pick(context)?.let { (id, pass) ->
            if (username.isNotEmpty() || password.isNotEmpty()) return@let
            username = id
            password = pass
            picked = id to pass
        }
    }

    /** VTOP let the user in: offers to save what they typed (unless it came from the password manager), then goes on. */
    fun signedIn() {
        val typed = username.trim() to password
        if (typed != picked) SavedPasswords.offerSave(context, typed.first, typed.second)
        onSignedIn()
    }

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
                    is SignIn.Done -> signedIn()
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

    // The fields sit on their segments (Account's look): no outline at rest, the primary one while typing.
    val tones = AppTheme.tones
    val fieldColors = OutlinedTextFieldDefaults.colors(
        unfocusedBorderColor = Color.Transparent,
        disabledBorderColor = Color.Transparent,
        errorBorderColor = Color.Transparent,
        disabledContainerColor = Color.Transparent,
    )
    val circle = @Composable { icon: ImageVector, error: Boolean ->
        Box(Modifier.size(42.dp).background(if (error) tones.danger.subtle else scheme.secondaryContainer, CircleShape), Alignment.Center) {
            Icon(icon, null, Modifier.size(20.dp), tint = if (error) tones.danger.base else scheme.onSecondaryContainer)
        }
    }
    val segment = ListItemDefaults.segmentedShapes(1, 3).shape
    fun segmentModifier(error: Boolean) = Modifier.fillMaxWidth().heightIn(min = 68.dp).clip(segment).background(cardColor)
        .then(if (error) Modifier.border(2.dp, scheme.error, segment) else Modifier)

    BoxWithConstraints(Modifier.fillMaxSize().background(scheme.background).imePadding().testTag("screen.login")) {
        Column(
            Modifier.verticalScroll(rememberScrollState()).fillMaxWidth().heightIn(min = maxHeight).systemBarsPadding()
                .padding(horizontal = Space.xl, vertical = Space.xxl),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center,
        ) {
            Column(Modifier.widthIn(max = 420.dp).fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                // The mark with two small companions: a calendar and a tick.
                Box(Modifier.size(width = 148.dp, height = 104.dp)) {
                    Box(Modifier.align(Alignment.Center).size(88.dp).background(scheme.primaryContainer, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
                        Icon(Lucide.GraduationCap, null, Modifier.size(40.dp), tint = scheme.onPrimaryContainer)
                    }
                    Box(Modifier.align(Alignment.BottomStart).size(44.dp).background(tones.accentTone.subtle, MaterialShapes.Cookie6Sided.toShape()), Alignment.Center) {
                        Icon(Lucide.CalendarDays, null, Modifier.size(20.dp), tint = tones.accentTone.base)
                    }
                    Box(Modifier.align(Alignment.TopEnd).size(40.dp).background(tones.success.subtle, MaterialShapes.Cookie4Sided.toShape()), Alignment.Center) {
                        Icon(Lucide.Check, null, Modifier.size(18.dp), tint = tones.success.base)
                    }
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
                // One segmented block of the two fields, as on the Account page.
                Column(Modifier.clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                Box(segmentModifier(usernameError != null || rejected), Alignment.CenterStart) {
                OutlinedTextField(
                    username,
                    {
                        username = it
                        usernameError = null
                        rejected = false
                    },
                    Modifier.fillMaxWidth().padding(4.dp).testTag("login.username").semantics { contentType = ContentType.Username },
                    enabled = !signingIn,
                    label = { Text("Username") },
                    leadingIcon = { circle(Lucide.User, usernameError != null || rejected) },
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
                    colors = fieldColors,
                )
                }
                Box(segmentModifier(passwordError != null), Alignment.CenterStart) {
                PasswordField(
                    password,
                    {
                        password = it
                        passwordError = null
                        rejected = false
                    },
                    "Password",
                    "login.password",
                    Modifier.fillMaxWidth().padding(4.dp).focusRequester(passwordFocus),
                    enabled = !signingIn,
                    isError = passwordError != null,
                    leadingIcon = { circle(Lucide.Lock, passwordError != null) },
                    supportingText = passwordError?.let { { Text(it, Modifier.testTag("login.password.error")) } },
                    keyboardActions = KeyboardActions(onDone = { submit() }),
                    shape = MaterialTheme.shapes.large,
                    colors = fieldColors,
                )
                }
                }
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
                Row(
                    Modifier.fillMaxWidth().background(scheme.surfaceContainerHighest, MaterialTheme.shapes.large).padding(Space.md),
                    horizontalArrangement = Arrangement.spacedBy(Space.sm),
                ) {
                    Icon(Lucide.Info, null, Modifier.size(18.dp), tint = scheme.onSurfaceVariant)
                    Text(
                        "Trouble signing in? Check that you can sign in on the VTOP website, then try again here.",
                        style = MaterialTheme.typography.bodySmall,
                        color = scheme.onSurfaceVariant,
                    )
                }
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
                            signedIn()
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
                scope.logAction("login.cancel_otp", "ui.login", "otp cancelled")
            },
        )
    }
}
