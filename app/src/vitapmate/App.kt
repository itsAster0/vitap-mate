package vitapmate

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
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.TriangleAlert
import vitapmate.core.Core
import vitapmate.data.AccountState
import vitapmate.data.AppCore
import vitapmate.data.InspectorOpener
import vitapmate.data.LocalInspector
import vitapmate.data.SemesterChoice
import vitapmate.data.Settings
import vitapmate.data.attempt
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.features.login.ChooseSemester
import vitapmate.features.login.FetchOtpOverlay
import vitapmate.features.login.LoginScreen
import vitapmate.features.login.MailForwarding
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.DownloadFolderPrompt
import vitapmate.ui.EmptyState
import vitapmate.ui.LocalSignIn
import vitapmate.ui.ToastHost

/**
 * The [AppSession]: signed out, the login screen; signed in without a semester picked:
 * [ChooseSemester] (the first time, [Onboarding] after it); then the shell. Any screen's
 * "Details" opens a flow here.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun App() {
    var session by remember { mutableStateOf<AppSession>(AppSession.Booting) }
    val on = { e: SessionEvent -> session = next(session, e) }
    LaunchedEffect(Unit) {
        attempt { coreFlow("app_start", Core::signedInUser) }
            .fold({ on(SessionEvent.Booted(it)) }, { on(SessionEvent.BootError(it)) })
    }
    val signedIn = session.signedIn
    // After signing in, the avatar needs the new name.
    LaunchedEffect(signedIn, accountVersion) {
        if (signedIn && (session.user == null || accountVersion > 0)) {
            on(SessionEvent.UserRead(orNull { coreFlow("app_user", Core::signedInUser) }))
        }
    }
    // The saved pick, read once per sign-in before choosing between the picker and the shell;
    // then each time the pick comes or goes.
    LaunchedEffect(signedIn) {
        if (!signedIn) return@LaunchedEffect
        AccountState.rejected = attempt {
            coreFlow("account.check") { c, id -> c.savedAccount(id)?.rejected == true }
        }.getOrDefault(false)
        AccountState.demo = attempt { coreFlow("account.demo", Core::isDemo) }.getOrDefault(false)
        flow("semesters.check") { SemesterChoice.readCache(it) }
        on(SessionEvent.AccountRead(SemesterChoice.selected != null, Settings.onboarded))
        snapshotFlow { SemesterChoice.selected != null }.collect { on(SessionEvent.PickChanged(it, Settings.onboarded)) }
    }
    val nav = rememberNav()
    val deviceAuth = rememberDeviceAuth()
    // A problem's "Sign in": a rejected password is fixed on the Account page in place (the
    // cache stays); with no saved account the login screen signs in again.
    val toLogin = {
        Log.info("ui.shell", "sign in again")
        AccountState.rejected = false
        nav.goTab(navTabs.first())
        on(SessionEvent.SignedOut)
    }
    val signIn = {
        if (AccountState.rejected) {
            Log.info("ui.shell", "update password")
            deviceAuth.confirm("Account") { nav.push(Route.Screen(ACCOUNT)) }
        } else {
            toLogin()
        }
    }
    // Update priority 5: nothing of the app until it's updated.
    if (PlayUpdate.blocked) return UpdateRequired()
    CompositionLocalProvider(
        LocalNav provides nav,
        LocalInspector provides InspectorOpener { nav.push(Route.FlowLog(it)) },
        LocalSignIn provides signIn,
        LocalUser provides session.user,
    ) {
        Box(Modifier.fillMaxSize().background(MaterialTheme.colorScheme.background).semantics { testTagsAsResourceId = true }) {
            if (signedIn) {
                // After a sign-in or on opening signed in, while the session is likely live.
                // The address may have moved: what Settings shows is read again after.
                LaunchedEffect(Unit) {
                    AppCore.keepForwarding { MailForwarding.load("keep") }
                    AppCore.proveInBackground()
                }
            }
            when (val s = session) {
                AppSession.Booting, is AppSession.LoadingAccount -> {}
                is AppSession.BootFailed -> EmptyState(Lucide.TriangleAlert, "Couldn't start", Modifier.align(Alignment.Center), message = "${s.error}")
                AppSession.SignedOut -> {
                    LoginScreen {
                        on(SessionEvent.SignedIn)
                        // Signed in over another student: leaves their address now, not at the next start.
                        AppCore.leaveOldForwarding()
                    }
                    // Signed out, the only screen over the login is a flow opened from "Details".
                    (nav.top as? Route.FlowLog)?.let {
                        BackHandler { nav.pop() }
                        FlowScreen(it.flow)
                    }
                }
                is AppSession.PickingSemester -> ChooseSemester()
                is AppSession.Onboarding -> Onboarding {
                    Log.info("ui.shell", "onboarding done")
                    Settings.onboarded = true
                    on(SessionEvent.OnboardingDone)
                }
                is AppSession.Ready -> Shell(nav, onSignedOut = toLogin)
            }
            FetchOtpOverlay()
            UpdatePillHost()
            ToastHost()
            DownloadFolderPrompt()
        }
    }
}
