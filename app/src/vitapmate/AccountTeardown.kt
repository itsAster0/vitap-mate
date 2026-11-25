package vitapmate

import android.content.Context
import android.webkit.CookieManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.data.AppCore
import vitapmate.data.AppScope
import vitapmate.data.Gmail
import vitapmate.data.OtpPrompt
import vitapmate.data.PageFetches
import vitapmate.data.Settings
import vitapmate.data.attempt
import vitapmate.features.vtop.forgetRecentVtopPages
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AvatarPhoto

/**
 * The one way a student's things leave the phone: Settings' sign-out ([signOut]) and Account's
 * switch to another student ([forgetStudent]). Both run on [AppScope], so leaving the screen
 * midway can't half-finish them.
 */
object AccountTeardown {
    /**
     * Rust's sign-out (the server, then the phone, then the forwarding addresses left behind),
     * then what Kotlin keeps for the student, on the main thread; [then] runs once all of it is done.
     */
    fun signOut(context: Context, then: () -> Unit) {
        val app = context.applicationContext
        AppScope.launch {
            flow("sign_out") { f ->
                attempt { withContext(Dispatchers.IO) { AppCore.await().signOut(f, false) } }
                    .onFailure { Log.error("ui.settings", "sign out failed: ${it::class.simpleName}", f) }
                forgetStudent(app, f)
            }
            // Rust cleared VTOP's cookies; the web view may hold others from this session.
            attempt { CookieManager.getInstance().removeAllCookies(null) }
            then()
        }
    }

    /**
     * What Kotlin keeps for the student who signed out or was replaced: a collapsed OTP prompt
     * and running page fetches (not over the login screen, not into the next account), their
     * photo, Gmail grant and the sync built on it, and their recent VTOP pages. The new
     * student's VTOP cookies are already in place on a switch, so cookies are [signOut]'s alone.
     */
    suspend fun forgetStudent(context: Context, flow: String) {
        OtpPrompt.cancel()
        PageFetches.cancelAll()
        AvatarPhoto.remove()
        attempt { Gmail.disconnect(flow) }.onFailure { Log.warn("ui.account", "gmail not disconnected: ${it::class.simpleName}", flow) }
        Settings.forgetStudent()
        BackgroundSync.apply(context)
        forgetRecentVtopPages(context)
    }
}
