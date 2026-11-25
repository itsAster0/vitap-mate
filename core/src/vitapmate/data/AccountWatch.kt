package vitapmate.data

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import vitapmate.core.AccountListener
import vitapmate.core.AccountStatus
import vitapmate.core.Standing

/**
 * Rust's [AccountStatus] (`Core.watchAccount`): who is signed in, and where their account
 * stands. The app's phase (the login screen or the shell) and what Kotlin keeps for a student
 * follow [heard]; screens draw [status]: when VTOP rejected the saved password, Settings shows
 * "Needs attention" and highlights Your account, and a sign-in problem opens the
 * Account page instead of the login screen, so nothing cached is dropped; the demo account
 * hides what needs a real VTOP session or the server. Only Rust changes it.
 */
object AccountWatch : AccountListener {
    private val told = MutableStateFlow<AccountStatus?>(null)

    /** Each status as Rust tells it, in order, on any thread; null until its first word. */
    val heard: StateFlow<AccountStatus?> = told.asStateFlow()

    /** [heard] for Compose, moved to the main thread. */
    var status by mutableStateOf<AccountStatus>(AccountStatus.SignedOut)
        private set

    /** The signed-in student's username; null signed out. */
    val username: String? get() = (status as? AccountStatus.SignedIn)?.username

    /** VTOP rejected the saved password (on this phone or on the server). */
    val rejected: Boolean get() = (status as? AccountStatus.SignedIn)?.standing is Standing.PasswordRejected

    /** Signed in as the demo account. */
    val demo: Boolean get() = (status as? AccountStatus.SignedIn)?.standing == Standing.Demo

    override fun changed(status: AccountStatus) {
        told.value = status
        AppScope.launch { this@AccountWatch.status = status }
    }
}
