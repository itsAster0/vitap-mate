package vitapmate

import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.input.TextFieldLineLimits
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.toShape
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Eye
import com.composables.icons.lucide.KeyRound
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.LogOut
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.User
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.core.SignIn
import vitapmate.data.AccountWatch
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.PauseReason
import vitapmate.data.SavedPasswords
import vitapmate.data.Settings
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppDialog
import vitapmate.ui.AppTheme
import vitapmate.ui.PasswordField
import vitapmate.ui.StatusChip
import vitapmate.ui.heroCookie
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast

/** Route id of Settings → Account. */
const val ACCOUNT = "account"

/** Bumped when the saved account changes (Account, Developer, another student): Settings reads the username again. */
var accountVersion by mutableIntStateOf(0)

/**
 * The saved username and password (the Flutter app's Update Credentials). Save signs in to VTOP
 * with them, the OTP prompt included, and keeps them only once VTOP accepts; a new password also
 * clears the "VTOP rejected the saved password" stop ([AccountWatch]). Opened from Settings or
 * from a sign-in problem's "Update password", without the screen lock: typing a new password
 * needs none, and the saved one is filled in and shown only after it ([DeviceAuth]). Another
 * username is another account. Below: the server account ([AccountServer]), and signing out.
 */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun AccountScreen() {
    val scope = rememberCoroutineScope()
    val focus = LocalFocusManager.current
    val haptics = rememberHaptics()
    val context = LocalContext.current
    val saved = LocalUser.current.orEmpty()
    var username by remember(saved) { mutableStateOf(saved) }
    var password by remember { mutableStateOf("") }
    var saving by remember { mutableStateOf(false) }
    val rejected = AccountWatch.rejected
    val deviceAuth = rememberDeviceAuth()
    // The saved password: read only after the screen lock, then filled in and shown.
    var savedPassword by remember { mutableStateOf<String?>(null) }
    fun reveal() = deviceAuth.confirm("Show the saved password") {
        scope.launch {
            savedPassword = orNull { coreFlow("account.read", Core::savedPassword) }
            savedPassword?.let { if (password.isEmpty()) password = it }
        }
    }
    val ready = !saving && username.isNotBlank() && password.isNotEmpty()

    fun save() {
        if (!ready) return
        focus.clearFocus()
        val other = !username.trim().equals(saved, ignoreCase = true)
        val changed = other || password != savedPassword
        scope.launch {
            saving = true
            var flowId = ""
            try {
                val result = flow("account.save") { id ->
                    flowId = id
                    Log.info("ui.account", "save", id, mapOf("other_account" to "$other"))
                    withContext(Dispatchers.IO) { AppCore.await().signIn(id, username.trim(), password) }
                }
                val done = when (result) {
                    is SignIn.Done -> true
                    // The app's OTP sheet takes it from here; saved once the code is accepted.
                    is SignIn.NeedsOtp -> {
                        // VTOP accepted the password before asking for the OTP; Rust already kept
                        // it for the saved student, not yet for another one (saved with the code).
                        if (!other) savedPassword = password
                        OtpPrompt.await(flowId)
                    }
                }
                Log.info("ui.account", "save result", flowId, mapOf("done" to "$done"))
                if (done) {
                    // Paused over the old password: the next run can try again.
                    Settings.syncHealth = Settings.syncHealth.resumedIf { it == PauseReason.PasswordRejected }
                    savedPassword = password
                    // Another student: the app lets go of the last one's things as Rust reports
                    // the switch ([AccountTeardown.follow]), wherever this page is by then.
                    if (!other) accountVersion++
                    haptics.confirm()
                    // Keeps the phone's password manager in step with what VTOP just accepted.
                    if (changed) SavedPasswords.offerSave(context, username.trim(), password)
                    showToast("Account saved", "VTOP accepted the new details.")
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Throwable) {
                haptics.reject()
                Log.warn("ui.account", "save failed", flowId, mapOf("error" to "${e::class.simpleName}"))
                if (e is CoreException.InvalidCredentials) {
                    showToast("Not saved", "VTOP didn't accept this username and password.")
                } else {
                    problemOf(e, flowId).let { showToast(it.title, it.message) }
                }
            } finally {
                saving = false
            }
        }
    }

    // The fields sit on their segments: no outline at rest, the primary one while typing.
    val scheme = MaterialTheme.colorScheme
    val fieldColors = OutlinedTextFieldDefaults.colors(
        unfocusedBorderColor = Color.Transparent,
        disabledBorderColor = Color.Transparent,
        errorBorderColor = Color.Transparent,
        disabledContainerColor = Color.Transparent,
    )
    val circle = @Composable { icon: ImageVector, error: Boolean ->
        Box(Modifier.size(42.dp).background(if (error) AppTheme.tones.danger.subtle else scheme.secondaryContainer, CircleShape), Alignment.Center) {
            Icon(icon, null, Modifier.size(20.dp), tint = if (error) AppTheme.tones.danger.base else scheme.onSecondaryContainer)
        }
    }
    val segment = ListItemDefaults.segmentedShapes(1, 3).shape
    val passwordError = rejected && password == (savedPassword ?: "")

    val server = rememberAccountServer()
    var signingOut by remember { mutableStateOf<Boolean?>(null) }

    Page("Account", ACCOUNT) { padding ->
        LazyColumn(contentPadding = padding) {
            if (saved.isNotBlank()) item { AccountHeader(saved, rejected, server.chip()) }
            item {
                Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp)) {
                    // One segmented block of the two fields.
                    Column(Modifier.clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                        Box(Modifier.fillMaxWidth().heightIn(min = 68.dp).clip(segment).background(AppTheme.colors.card), Alignment.CenterStart) {
                            OutlinedTextField(
                                vitapmate.ui.rememberSyncedTextFieldState(username) { username = it },
                                Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 4.dp).testTag("account.username").semantics { contentType = ContentType.Username },
                                enabled = !saving,
                                label = { Text("Username") },
                                leadingIcon = { circle(Lucide.User, false) },
                                keyboardOptions = KeyboardOptions(
                                    capitalization = KeyboardCapitalization.None,
                                    autoCorrectEnabled = false,
                                    keyboardType = KeyboardType.Ascii,
                                    imeAction = ImeAction.Next,
                                ),
                                lineLimits = TextFieldLineLimits.SingleLine,
                                shape = MaterialTheme.shapes.large,
                                colors = fieldColors,
                            )
                        }
                        Box(
                            Modifier.fillMaxWidth().heightIn(min = 68.dp).clip(segment).background(AppTheme.colors.card)
                                .then(if (passwordError) Modifier.border(2.dp, scheme.error, segment) else Modifier),
                            Alignment.CenterStart,
                        ) {
                            PasswordField(
                                password,
                                { password = it },
                                "Password",
                                "account.password",
                                Modifier.fillMaxWidth().padding(horizontal = 4.dp, vertical = 4.dp),
                                enabled = !saving,
                                isError = passwordError,
                                initiallyShown = savedPassword != null,
                                leadingIcon = { circle(Lucide.Lock, passwordError) },
                                onKeyboardAction = { save() },
                                shape = MaterialTheme.shapes.large,
                                colors = fieldColors,
                            )
                        }
                    }
                    if (savedPassword == null && password.isEmpty()) {
                        TextButton(::reveal, Modifier.padding(start = 8.dp, top = 4.dp).testTag("account.reveal"), enabled = !saving) {
                            Icon(Lucide.Eye, null, Modifier.size(18.dp))
                            Text("Show the saved password", Modifier.padding(start = 8.dp))
                        }
                    }
                    if (rejected) {
                        Text(
                            "VTOP rejected the saved password. Enter your current VTOP password.",
                            Modifier.padding(start = 20.dp, top = 8.dp, end = 20.dp),
                            style = MaterialTheme.typography.bodySmall,
                            color = scheme.error,
                        )
                    }
                    Button(
                        ::save,
                        modifier = Modifier.fillMaxWidth().padding(top = 22.dp).heightIn(min = 62.dp).testTag("account.save"),
                        enabled = ready,
                        shapes = ButtonDefaults.shapes(),
                    ) {
                        AnimatedContent(saving, contentAlignment = Alignment.Center, label = "account.save") { busy ->
                            if (busy) {
                                LoadingIndicator(Modifier.size(26.dp).testTag("account.loading"))
                            } else {
                                Text("Save", style = MaterialTheme.typography.titleMedium)
                            }
                        }
                    }
                }
            }
            accountServer(server)
            item {
                ListGroup("Sign out", footer = "Signing out removes the saved account and cached pages from this phone.") {
                    GroupItem("Sign out on this phone", "account.sign_out", Lucide.LogOut, onClick = { signingOut = false })
                    if (server.linked) GroupItem(
                        "Sign out everywhere",
                        "account.sign_out_everywhere",
                        Lucide.LogOut,
                        "Every phone on your account, this one too",
                        tone = AppTheme.tones.danger,
                        onClick = { signingOut = true },
                    )
                }
            }
        }
    }

    server.actions.Dialog(server::done)
    signingOut?.let { everywhere ->
        AppDialog(
            { signingOut = null },
            if (everywhere) "Sign out everywhere?" else "Sign out?",
            "Sign out",
            {
                signingOut = null
                AccountTeardown.signOut(context, everywhere)
            },
            icon = Lucide.LogOut,
            danger = true,
            confirmTag = "account.sign_out.confirm",
        ) {
            Text(
                if (everywhere) "Every phone on your server account is signed out of the server and stops getting session requests and forwarded OTPs. The saved account and cached pages are removed from this phone; the other phones keep theirs."
                else "The saved account and cached pages are removed from this phone.",
            )
        }
    }
}

/** Which account this is ([username]) and its state, in the Settings header's hero style; [server] is the server link's chip. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun AccountHeader(username: String, rejected: Boolean, server: (@Composable () -> Unit)?) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    Row(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).clip(MaterialTheme.shapes.extraLarge)
            .background(tones.accentTone.subtle).heroCookie().padding(18.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.size(78.dp).background(scheme.primaryContainer, MaterialShapes.Cookie9Sided.toShape()), Alignment.Center) {
            Icon(Lucide.KeyRound, null, Modifier.size(30.dp), tint = scheme.onPrimaryContainer)
        }
        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(username, style = MaterialTheme.typography.titleLargeEmphasized, color = scheme.onSurface, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Text("The account this phone signs in to VTOP with", style = MaterialTheme.typography.bodyMedium, color = tones.accentTone.onSubtle)
            Row(Modifier.padding(top = 6.dp), horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                if (rejected) StatusChip("Needs attention", tones.warning)
                else StatusChip("Signed in", tones.success)
                server?.invoke()
            }
        }
    }
}
