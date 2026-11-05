package vitapmate

import androidx.compose.runtime.LaunchedEffect
import vitapmate.data.AccountState
import vitapmate.ui.rememberHaptics
import androidx.compose.animation.AnimatedContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Lock
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.User
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.CoreException
import vitapmate.core.SignIn
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.PageFetches
import vitapmate.data.SemesterChoice
import vitapmate.data.problemOf
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AvatarPhoto
import vitapmate.ui.PasswordField
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
    val saved = LocalUser.current.orEmpty()
    var username by remember(saved) { mutableStateOf(saved) }
    var password by remember { mutableStateOf("") }
    var saving by remember { mutableStateOf(false) }
    val rejected = AccountState.rejected
    // The saved password, filled in (this page opens behind the phone's screen lock); shown
    // when VTOP rejected it, so the student can see what to correct.
    var savedPassword by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(Unit) {
        val account = runCatching { flow("account.read") { id -> withContext(Dispatchers.IO) { AppCore.await().savedAccount(id) } } }.getOrNull()
        savedPassword = account?.password
        if (password.isEmpty()) password = account?.password.orEmpty()
    }
    val ready = !saving && username.isNotBlank() && password.isNotEmpty()

    fun save() {
        if (!ready) return
        focus.clearFocus()
        val other = !username.trim().equals(saved, ignoreCase = true)
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
                        val before = OtpPrompt.generation
                        OtpPrompt.ask(result.message)
                        snapshotFlow { OtpPrompt.message }.first { it == null }
                        OtpPrompt.generation != before
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

    Page("Account", ACCOUNT) { padding ->
        LazyColumn(contentPadding = padding) {
            item {
                Column(Modifier.fillMaxWidth().padding(horizontal = 32.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    OutlinedTextField(
                        username,
                        { username = it },
                        Modifier.fillMaxWidth().testTag("account.username").semantics { contentType = ContentType.Username },
                        enabled = !saving,
                        label = { Text("Username") },
                        leadingIcon = { Icon(Lucide.User, null) },
                        keyboardOptions = KeyboardOptions(
                            capitalization = KeyboardCapitalization.None,
                            autoCorrectEnabled = false,
                            keyboardType = KeyboardType.Ascii,
                            imeAction = ImeAction.Next,
                        ),
                        singleLine = true,
                        shape = MaterialTheme.shapes.large,
                    )
                    PasswordField(
                        password,
                        { password = it },
                        "Password",
                        "account.password",
                        Modifier.fillMaxWidth(),
                        enabled = !saving,
                        isError = rejected && password == savedPassword,
                        supportingText = if (rejected) {
                            { Text("VTOP rejected this password. Enter your current VTOP password.") }
                        } else {
                            null
                        },
                        initiallyShown = rejected,
                        leadingIcon = { Icon(Lucide.Lock, null) },
                        keyboardActions = KeyboardActions(onDone = { save() }),
                        shape = MaterialTheme.shapes.large,
                    )
                    Button(
                        ::save,
                        modifier = Modifier.fillMaxWidth().padding(top = 12.dp).heightIn(min = ButtonDefaults.MediumContainerHeight).testTag("account.save"),
                        enabled = ready,
                        shapes = ButtonDefaults.shapes(),
                    ) {
                        AnimatedContent(saving, contentAlignment = Alignment.Center, label = "account.save") { busy ->
                            if (busy) {
                                LoadingIndicator(Modifier.size(24.dp).testTag("account.loading"))
                            } else {
                                Text("Save", style = MaterialTheme.typography.titleSmall)
                            }
                        }
                    }
                }
            }
        }
    }
}
