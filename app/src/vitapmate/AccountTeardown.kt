package vitapmate

import android.content.Context
import android.webkit.CookieManager
import java.util.concurrent.atomic.AtomicBoolean
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.AccountStatus
import vitapmate.data.AccountWatch
import vitapmate.data.AppCore
import vitapmate.data.AppScope
import vitapmate.data.Gmail
import vitapmate.data.OtpPrompt
import vitapmate.data.PageFetches
import vitapmate.data.SemesterChoice
import vitapmate.data.Settings
import vitapmate.data.attempt
import vitapmate.features.login.MailForwarding
import vitapmate.features.vtop.forgetRecentVtopPages
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AvatarPhoto

/**
 * The one way a student's things leave the phone: Settings' sign-out ([signOut]) and another
 * student signing in ([follow]). Both run on [AppScope], so leaving the screen midway can't
 * half-finish them.
 */
object AccountTeardown {
    private val following = AtomicBoolean(false)
    private val settledAs = MutableStateFlow<String?>(null)

    /** The signed-in student once the last one's things are gone ([follow]); null signed out. */
    val settled: StateFlow<String?> = settledAs.asStateFlow()

    /**
     * Follows Rust's [AccountWatch.heard] for the life of the process, once: when another
     * student than [Settings.student] signs in, wherever their code was accepted (the login
     * screen, Account, a page's OTP prompt), the last one's things go ([forgetStudent]), the
     * new one's semesters are read, the last one's forwarding address is left and the new one
     * is proved; then [settled] names them. Signing in as the same student again keeps it all.
     */
    fun follow(context: Context) {
        if (!following.compareAndSet(false, true)) return
        val app = context.applicationContext
        AppScope.launch {
            AccountWatch.heard.filterNotNull().collect { status ->
                val student = (status as? AccountStatus.SignedIn)?.username
                val last = Settings.student
                if (student != null && last != null && !last.equals(student, ignoreCase = true)) {
                    flow("account.switch") { f ->
                        Log.info("account", "another student signed in", f)
                        forgetStudent(app, f)
                        SemesterChoice.readCache(f)
                    }
                    // The last student's Mailbox would keep sending this phone their OTPs.
                    AppCore.leaveOldForwarding()
                    AppCore.proveInBackground()
                    accountVersion++
                }
                if (student != null) Settings.student = student
                settledAs.value = student
            }
        }
    }

    /**
     * Rust's sign-out (the server, for every phone on the account with [everywhere], then the
     * phone, then the forwarding addresses left behind), then what Kotlin keeps for the student,
     * on the main thread. The app follows Rust's status back to the login screen.
     */
    fun signOut(context: Context, everywhere: Boolean = false) {
        val app = context.applicationContext
        AppScope.launch {
            flow("sign_out") { f ->
                Log.info("ui.settings", "sign out", f, mapOf("everywhere" to "$everywhere"))
                attempt { withContext(Dispatchers.IO) { AppCore.await().signOut(f, everywhere) } }
                    .onFailure { Log.error("ui.settings", "sign out failed: ${it::class.simpleName}", f) }
                forgetStudent(app, f)
            }
            // Rust cleared VTOP's cookies; the web view may hold others from this session.
            attempt { CookieManager.getInstance().removeAllCookies(null) }
        }
    }

    /**
     * What Kotlin keeps for the student who signed out or was replaced: a collapsed OTP prompt
     * and running page fetches (not over the login screen, not into the next account), their
     * photo, Gmail grant and the sync built on it, and their recent VTOP pages. The new
     * student's VTOP cookies are already in place on a switch, so cookies are [signOut]'s alone.
     */
    private suspend fun forgetStudent(context: Context, flow: String) {
        OtpPrompt.cancel()
        PageFetches.cancelAll()
        AvatarPhoto.remove()
        attempt { Gmail.disconnect(flow) }.onFailure { Log.warn("ui.account", "gmail not disconnected: ${it::class.simpleName}", flow) }
        Settings.forgetStudent()
        // Rust already cleared forwarding.json on sign-out or switch; refresh before sync or the next student reads it.
        MailForwarding.load("forget_student")
        BackgroundSync.apply(context)
        forgetRecentVtopPages(context)
    }
}
