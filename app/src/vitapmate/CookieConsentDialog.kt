package vitapmate

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Checkbox
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.withStyle
import com.composables.icons.lucide.Cookie
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.launch
import vitapmate.core.CookieAnswer
import vitapmate.core.CookieKind
import vitapmate.data.CookieConsent
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.ui.Space
import vitapmate.ui.showToast

/**
 * The first of [CookieConsent.pending] as the "Send your VTOP cookie?" dialog; dismissing it is
 * Cancel. "Don't ask again" starts ticked and counts only with Send.
 */
@Composable
fun CookieConsentDialog() {
    val request = CookieConsent.pending.firstOrNull() ?: return
    var remember by remember(request) { mutableStateOf(true) }
    val answer = { send: Boolean -> CookieConsent.answer(request, CookieAnswer(send, send && remember)) }
    val password = if (request.action.contains("password", ignoreCase = true)) ", and your password" else ""
    AlertDialog(
        onDismissRequest = { answer(false) },
        icon = { Icon(Lucide.Cookie, null) },
        title = { Text("Send your VTOP cookie?") },
        text = {
            // No taps through an overlay another app draws over the dialog (tapjacking).
            val view = LocalView.current
            DisposableEffect(view) { view.filterTouchesWhenObscured = true; onDispose {} }
            Column {
                Text(
                    buildAnnotatedString {
                        withStyle(SpanStyle(fontWeight = FontWeight.SemiBold)) { append(request.action) }
                        append(" needs your VTOP sign-in cookie$password. It goes to ")
                        withStyle(SpanStyle(fontWeight = FontWeight.SemiBold)) { append(request.server) }
                        append(" and our engine, which uses it to reach VTOP for you. ")
                        append(request.kind.line)
                    },
                )
                Row(
                    Modifier.fillMaxWidth().padding(top = Space.md)
                        .toggleable(remember, role = Role.Checkbox) { remember = it }
                        .testTag("consent.remember"),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Checkbox(remember, null)
                    Text("Don't ask again for ${request.kind.label}", style = MaterialTheme.typography.bodyMedium)
                }
            }
        },
        confirmButton = { TextButton({ answer(true) }, Modifier.testTag("consent.send")) { Text("Send") } },
        dismissButton = { TextButton({ answer(false) }, Modifier.testTag("consent.cancel")) { Text("Cancel") } },
    )
}

/** Settings → Connections: the kinds sent without asking, and a reset; hidden while there are none. */
@Composable
fun CookieConsentSetting() {
    val scope = rememberCoroutineScope()
    var version by remember { mutableIntStateOf(0) }
    // Read again after each dialog (a tick may have added a kind) and after a reset.
    val remembered by produceState(emptyList(), version, CookieConsent.pending.size) {
        value = orNull { coreFlow("settings.cookie_consent") { c, f -> c.cookieConsentRemembered(f) } }.orEmpty()
    }
    if (remembered.isEmpty()) return
    GroupItem(
        "Ask before sending your VTOP cookie",
        "settings.cookie_consent",
        Lucide.Cookie,
        "Not for: ${remembered.joinToString { it.label }}",
        trailing = {
            TextButton({
                scope.launch {
                    orNull { coreFlow("settings.cookie_reset") { c, f -> c.resetCookieConsent(f) } }?.let {
                        version++
                        showToast("Cookie", "You'll be asked every time")
                    }
                }
            }, Modifier.testTag("settings.cookie_reset")) { Text("Ask every time again") }
        },
    )
}

private val CookieKind.label get() = when (this) {
    CookieKind.ROUTE -> "server and proxy routes"
    CookieKind.ACCOUNT -> "your server account"
    CookieKind.HANDOVER -> "connected apps"
}

private val CookieKind.line get() = when (this) {
    CookieKind.ROUTE -> "It is used for this fetch, with your password if VTOP needs a new sign-in, and not kept."
    CookieKind.ACCOUNT -> "It only proves who you are; the server doesn't keep it."
    CookieKind.HANDOVER -> "A connected app (browser extension or AI app) gets a live VTOP session."
}
