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
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
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
import com.composables.icons.lucide.KeyRound
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.User
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.Core
import vitapmate.core.CoreException
import vitapmate.core.SignIn
import vitapmate.data.AccountState
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.PageFetches
import vitapmate.data.SavedPasswords
import vitapmate.data.SemesterChoice
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.AvatarPhoto
import vitapmate.ui.PasswordField
import vitapmate.ui.StatusChip
import vitapmate.ui.heroCookie
import vitapmate.ui.rememberHaptics
import vitapmate.ui.showToast

/** Route id of Settings → Account (opened after [DeviceAuth]). */
const val ACCOUNT = "account"

/** Bumped when the saved account changes in place (Account, Developer): the app reads the username again. */
var accountVersion by mutableIntStateOf(0)

/**
 * The saved username and password (the Flutter app's Update Credentials). Save signs in to VTOP
 * with them, the OTP prompt included, and keeps them only once VTOP accepts; a new password also
 * clears the "VTOP rejected the saved password" stop ([AccountState]). Opened from Settings or
 * from a sign-in problem's "Update password"; the saved password is filled in and, when VTOP
 * rejected it, shown. Another username is another account.
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
    val rejected = AccountState.rejected
    // The saved password, filled in (this page opens behind the phone's screen lock); shown
    // when VTOP rejected it, so the student can see what to correct.
    var savedPassword by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(Unit) {
        val account = orNull { coreFlow("account.read", Core::savedAccount) }
        savedPassword = account?.password
        if (password.isEmpty()) password = account?.password.orEmpty()
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
                        // VTOP accepted the password before asking for the OTP; Rust already kept it.
                        AccountState.rejected = false
                        savedPassword = password
                        OtpPrompt.askAndWait(result.message)
                    }
                }
                Log.info("ui.account", "save result", flowId, mapOf("done" to "$done"))
                if (done) {
                    AccountState.rejected = false
                    savedPassword = password
                    if (other) {
                        PageFetches.cancelAll()
                        AvatarPhoto.remove()
                        flow("account.semesters") { SemesterChoice.readCache(it) }
                    }
                    accountVersion++
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
    val passwordError = rejected && password == savedPassword

    Page("Account", ACCOUNT) { padding ->
        LazyColumn(contentPadding = padding) {
            if (saved.isNotBlank()) item { AccountHeader(saved, rejected) }
            item {
                Column(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 12.dp)) {
                    // One segmented block of the two fields.
                    Column(Modifier.clip(MaterialTheme.shapes.extraLarge), verticalArrangement = Arrangement.spacedBy(ListItemDefaults.SegmentedGap)) {
                        Box(Modifier.fillMaxWidth().heightIn(min = 68.dp).clip(segment).background(AppTheme.colors.card), Alignment.CenterStart) {
                            OutlinedTextField(
                                username,
                                { username = it },
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
                                singleLine = true,
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
                                initiallyShown = rejected,
                                leadingIcon = { circle(Lucide.Lock, passwordError) },
                                keyboardActions = KeyboardActions(onDone = { save() }),
                                shape = MaterialTheme.shapes.large,
                                colors = fieldColors,
                            )
                        }
                    }
                    if (rejected) {
                        Text(
                            "VTOP rejected this password. Enter your current VTOP password.",
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
        }
    }
}

/** Which account this is ([username]) and its state, in the Settings header's hero style. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun AccountHeader(username: String, rejected: Boolean) {
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
            if (rejected) StatusChip("Needs attention", tones.warning, Modifier.padding(top = 6.dp))
            else StatusChip("Signed in", tones.success, Modifier.padding(top = 6.dp))
        }
    }
}
