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
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import vitapmate.core.AccountStatus
import vitapmate.data.AccountWatch
import vitapmate.data.AppCore
import vitapmate.data.InspectorOpener
import vitapmate.data.LocalInspector
import vitapmate.data.SemesterChoice
import vitapmate.data.Settings
import vitapmate.data.attempt
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
 * The [AppSession], following who Rust says is signed in: signed out, the login screen; signed
 * in without a semester picked: [ChooseSemester] (the first time, [Onboarding] after it); then
 * the shell. Any screen's "Details" opens a flow here.
 */
@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun App() {
    var session by remember { mutableStateOf<AppSession>(AppSession.Booting) }
    val on = { e: SessionEvent -> session = next(session, e) }
    // Who is signed in is Rust's: the phase follows its status from the start.
    LaunchedEffect(Unit) {
        attempt { AppCore.await() }.onFailure { on(SessionEvent.BootError(it)) }.onSuccess {
            AccountWatch.heard.filterNotNull().collect { on(SessionEvent.Account((it as? AccountStatus.SignedIn)?.username)) }
        }
    }
    val signedIn = session.signedIn
    val nav = rememberNav()
    // Signed out (here, or the saved account gone): the login screen, on the first tab after.
    LaunchedEffect(signedIn) {
        if (!signedIn && session !is AppSession.Booting) nav.goTab(navTabs.first())
    }
    // The saved pick, read once per student (after the last one's things went) before choosing
    // between the picker and the shell; then each time the pick comes or goes.
    val loading = (session as? AppSession.LoadingAccount)?.user
    LaunchedEffect(loading) {
        val user = loading ?: return@LaunchedEffect
        AccountTeardown.settled.first { it.equals(user, ignoreCase = true) }
        flow("semesters.check") { SemesterChoice.readCache(it) }
        on(SessionEvent.AccountRead(SemesterChoice.selected != null, setup()))
    }
    LaunchedEffect(signedIn) {
        if (!signedIn) return@LaunchedEffect
        snapshotFlow { SemesterChoice.selected != null }.collect { on(SessionEvent.PickChanged(it, setup())) }
    }
    // The setup page is kept pending until done, so a restart on it opens it again.
    LaunchedEffect(session is AppSession.Onboarding) {
        if (session is AppSession.Onboarding) Settings.onboardingPending = true
    }
    val deviceAuth = rememberDeviceAuth()
    // A problem's "Sign in": signed in (VTOP rejected the password, or turned the saved details
    // down), they are fixed on the Account page in place, so the cache stays; signed out, the
    // login screen is already up.
    val signIn: () -> Unit = {
        if (AccountWatch.username != null) {
            Log.info("ui.shell", "update password")
            deviceAuth.confirm("Account") { nav.push(Route.Screen(ACCOUNT)) }
        } else {
            Log.info("ui.shell", "sign in again")
            nav.goTab(navTabs.first())
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
                    // Rust's status moves the app on once signed in; addresses a sign-out couldn't
                    // leave (offline) are left now, not at the next start.
                    LoginScreen { AppCore.leaveOldForwarding() }
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
                    Settings.onboardingPending = false
                    on(SessionEvent.OnboardingDone)
                }
                is AppSession.Ready -> Shell(nav)
            }
            FetchOtpOverlay()
            UpdatePillHost()
            ToastHost()
            DownloadFolderPrompt()
        }
    }
}

/** How far this install got with the setup page ([Settings.onboarded], [Settings.onboardingPending]). */
private fun setup(): Setup = when {
    Settings.onboarded -> Setup.Done
    Settings.onboardingPending -> Setup.Pending
    else -> Setup.NotYet
}
