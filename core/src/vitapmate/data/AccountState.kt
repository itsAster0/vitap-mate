package vitapmate.data

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/**
 * VTOP rejected the saved password. Settings then shows "Needs attention" and highlights
 * Username and password, and a sign-in problem opens the Account page instead of the login
 * screen, so nothing cached is dropped. Read at start ([Core.savedAccount]), set by
 * [problemOf], cleared when the Account page saves a password VTOP accepts.
 */
object AccountState {
    var rejected by mutableStateOf(false)
}
