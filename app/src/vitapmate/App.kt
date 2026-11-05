package vitapmate

import vitapmate.data.AccountState
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.TriangleAlert
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.data.AppCore
import vitapmate.data.InspectorOpener
import vitapmate.data.LocalInspector
import vitapmate.data.SemesterChoice
import vitapmate.features.login.ChooseSemester
import vitapmate.features.login.FetchOtpOverlay
import vitapmate.features.login.LoginScreen
import vitapmate.observability.flow
import vitapmate.observability.Log
import vitapmate.ui.EmptyState
import vitapmate.ui.LocalSignIn
import vitapmate.ui.DownloadFolderPrompt
import vitapmate.ui.ToastHost

/**
 * Signed out: the login screen; signed in without a semester picked: [ChooseSemester]; then the
 * shell. Any screen's "Details" opens a flow here.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun App() {
    // null while the core starts; a failure shows instead of the app.
    var signedIn by remember { mutableStateOf<Result<Boolean>?>(null) }
    var user by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(Unit) {
        signedIn = runCatching {
            flow("app_start") { id -> withContext(Dispatchers.IO) { AppCore.await().signedInUser(id) }.also { user = it } != null }
        }
    }
    // After signing in, the avatar needs the new name.
    LaunchedEffect(signedIn, accountVersion) {
        if (signedIn?.getOrNull() == true && (user == null || accountVersion > 0)) {
            user = runCatching { flow("app_user") { id -> withContext(Dispatchers.IO) { AppCore.await().signedInUser(id) } } }.getOrNull()
        }
    }
    // The saved pick, read once per sign-in before choosing between the picker and the shell.
    var semesterRead by remember(signedIn) { mutableStateOf(false) }
    LaunchedEffect(signedIn) {
        if (signedIn?.getOrNull() == true) {
            AccountState.rejected = runCatching {
                flow("account.check") { id -> withContext(Dispatchers.IO) { AppCore.await().savedAccount(id)?.rejected == true } }
            }.getOrDefault(false)
            flow("semesters.check") { SemesterChoice.readCache(it) }
            semesterRead = true
        }
    }
    val nav = rememberNav()
    val deviceAuth = rememberDeviceAuth()
    // A problem's "Sign in": a rejected password is fixed on the Account page in place (the
    // cache stays); with no saved account the login screen signs in again.
    val toLogin = {
        Log.info("ui.shell", "sign in again")
        AccountState.rejected = false
        nav.goTab(navTabs.first())
        user = null
        signedIn = Result.success(false)
    }
    val signIn = {
        if (AccountState.rejected) {
            Log.info("ui.shell", "update password")
            deviceAuth.confirm("Account") { nav.push(Route.Screen(ACCOUNT)) }
        } else {
            toLogin()
        }
    }
    CompositionLocalProvider(
        LocalNav provides nav,
        LocalInspector provides InspectorOpener { nav.push(Route.FlowLog(it)) },
        LocalSignIn provides signIn,
        LocalUser provides user,
    ) {
        Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).semantics { testTagsAsResourceId = true }) {
            signedIn?.fold(
                onSuccess = { yes ->
                    if (yes) {
                        when {
                            !semesterRead -> {}
                            SemesterChoice.selected == null -> ChooseSemester()
                            else -> Shell(nav, onSignedOut = toLogin)
                        }
                    } else {
                        LoginScreen { signedIn = Result.success(true) }
                        // Signed out, the only screen over the login is a flow opened from "Details".
                        (nav.top as? Route.FlowLog)?.let {
                            BackHandler { nav.pop() }
                            FlowScreen(it.flow)
                        }
                    }
                },
                onFailure = {
                    EmptyState(Lucide.TriangleAlert, "Couldn't start", Modifier.align(Alignment.Center), message = "$it")
                },
            )
            FetchOtpOverlay()
            ToastHost()
            DownloadFolderPrompt()
        }
    }
}
