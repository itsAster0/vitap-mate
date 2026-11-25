package vitapmate

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.collectAsState
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
import vitapmate.core.CookiePayload
import vitapmate.data.CookieConsent
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.ui.AppDialog
import vitapmate.ui.DialogRows
import vitapmate.ui.Space
import vitapmate.ui.rememberHaptics

/**
 * The first of [CookieConsent.pending] as the "Send your VTOP cookie?" dialog; dismissing it is
 * Cancel. "Don't ask again" starts ticked and counts only with Send.
 */
@Composable
fun CookieConsentDialog() {
    val pending by CookieConsent.pending.collectAsState()
    val request = pending.firstOrNull() ?: return
    var remember by remember(request) { mutableStateOf(true) }
    val answer = { send: Boolean -> CookieConsent.answer(request, if (send) CookieAnswer.Allow(remember) else CookieAnswer.Deny) }
    val password = when (request.payload) {
        CookiePayload.COOKIE -> ""
        CookiePayload.COOKIE_AND_PASSWORD -> ", and your password"
    }
    AppDialog(
        { answer(false) },
        "Send your VTOP cookie?",
        "Send",
        { answer(true) },
        icon = Lucide.Cookie,
        confirmTag = "consent.send",
        dismissTag = "consent.cancel",
    ) {
        // No taps through an overlay another app draws over the dialog (tapjacking).
        val view = LocalView.current
        DisposableEffect(view) { view.filterTouchesWhenObscured = true; onDispose {} }
        Text(
            buildAnnotatedString {
                withStyle(SpanStyle(fontWeight = FontWeight.SemiBold)) { append(request.action) }
                append(" needs your VTOP sign-in cookie$password.")
            },
        )
        DialogRows(
            listOf(
                "Goes to" to "${request.server} and our engine, which uses it to reach VTOP for you",
                "Used for" to request.kind.line,
            ),
            Modifier.padding(top = Space.md),
        )
        Row(
            Modifier.fillMaxWidth().padding(top = Space.md)
                .toggleable(remember, role = Role.Switch) { remember = it }
                .testTag("consent.remember"),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("Don't ask again for ${request.kind.label}", Modifier.weight(1f))
            Switch(remember, null)
        }
    }
}

/**
 * "Ask before …" for one [kind] (Connection's routes, Connected apps' handover): on while its
 * "Don't ask again" isn't ticked; turning it off ticks it, as the dialog's tick would.
 */
@Composable
fun CookieAskSetting(kind: CookieKind, title: String, tag: String) {
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    var version by remember { mutableIntStateOf(0) }
    val pending by CookieConsent.pending.collectAsState()
    // Read again after each dialog (a tick may have added the kind) and after a change here.
    val remembered by produceState<Boolean?>(null, version, pending.size) {
        value = orNull { coreFlow("consent.read") { c, f -> c.cookieConsentRemembered(f) } }?.contains(kind)
    }
    val asks = remembered?.not() ?: return
    val set = { ask: Boolean ->
        haptics.toggle(ask)
        scope.launch {
            orNull {
                coreFlow(if (ask) "consent.ask" else "consent.skip") { c, f ->
                    if (ask) c.forgetCookieConsent(f, kind) else c.rememberCookieConsent(f, kind)
                }
            }
            version++
        }
        Unit
    }
    GroupItem(
        title,
        tag,
        Lucide.Cookie,
        if (asks) "Each time, saying where it goes" else "Sent without asking for ${kind.label}",
        onClick = { set(!asks) },
        trailing = { ItemSwitch(asks, { set(it) }) },
        on = asks,
    )
}

private val CookieKind.label get() = when (this) {
    CookieKind.ROUTE -> "server routes"
    CookieKind.ACCOUNT -> "your server account"
    CookieKind.HANDOVER -> "connected apps"
}

private val CookieKind.line get() = when (this) {
    CookieKind.ROUTE -> "It is used for this fetch, with your password if VTOP needs a new sign-in, and not kept."
    // The account proof (TLSNotary) sends no cookie; this kind now only covers the saved password.
    CookieKind.ACCOUNT -> "Your password goes to the server to keep sealed, so it can sign in for you."
    CookieKind.HANDOVER -> "A connected app (browser extension or AI app) gets a live VTOP session."
}
