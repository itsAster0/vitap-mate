package vitapmate.data

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.launch
import vitapmate.core.AccountListener
import vitapmate.core.AccountStatus

/**
 * Rust's [AccountStatus] (`Core.watchAccount`), as screens draw it: when VTOP rejected the saved
 * password, Settings shows "Needs attention" and highlights Username and password, and a sign-in
 * problem opens the Account page instead of the login screen, so nothing cached is dropped; the
 * demo account hides what needs a real VTOP session or the server. Only Rust changes it; the
 * value moves to the main thread before Compose reads it.
 */
object AccountWatch : AccountListener {
    var status by mutableStateOf<AccountStatus>(AccountStatus.SignedOut)
        private set

    /** VTOP rejected the saved password (on this phone or on the server). */
    val rejected: Boolean get() = status is AccountStatus.PasswordRejected

    /** Signed in as the demo account. */
    val demo: Boolean get() = status == AccountStatus.Demo

    override fun changed(status: AccountStatus) {
        AppScope.launch { this@AccountWatch.status = status }
    }
}
