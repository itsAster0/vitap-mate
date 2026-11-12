package vitapmate.features.login

import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.ChevronDown
import com.composables.icons.lucide.ChevronUp
import com.composables.icons.lucide.FileBraces
import com.composables.icons.lucide.LogIn
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.MailCheck
import com.composables.icons.lucide.TriangleAlert
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.data.AppCore
import vitapmate.data.Gmail
import vitapmate.data.OtpSource
import vitapmate.data.Settings
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.Button
import vitapmate.ui.ButtonVariant
import vitapmate.ui.OpenHaptic
import vitapmate.ui.Space
import vitapmate.ui.StatusChip
import vitapmate.ui.showToast

private const val SRC = "ui.gmail"

/** The Flutter app's `GmailOAuthGuidePage` steps 1–6; step 7 is the buttons below them. */
private val steps = listOf(
    Triple(
        "Create a Google Cloud project",
        "https://console.cloud.google.com/projectcreate",
        "Sign in with your personal Gmail (…@gmail.com), not your college account. Name it anything (vitap-mate), leave Location as \"No organization\", tap Create, then pick the new project in the picker at the top.",
    ),
    Triple(
        "Turn on the Gmail API",
        "https://console.cloud.google.com/apis/library/gmail.googleapis.com",
        "Check the picker at the top shows your project, then tap Enable.",
    ),
    Triple(
        "Set up the consent screen",
        "https://console.cloud.google.com/auth/branding",
        "Tap Get started. App name \"Vitap Mate\", support email your Gmail. Audience: External (Internal only works for Workspace organisations). Contact: your Gmail, tick the policy box, then Create.",
    ),
    Triple(
        "Add your college email as a test user",
        "https://console.cloud.google.com/auth/audience",
        "Under Test users tap Add users, enter your @vitapstudent.ac.in email and Save. Sharing the project with friends? Add their college emails too.",
    ),
    Triple(
        "Create a Desktop app client",
        "https://console.cloud.google.com/auth/clients",
        "Tap Create client, Application type Desktop app (not Android or Web), then Create. In the popup tap Download JSON right away: Google never shows the secret again. On a computer? Move the file to this phone.",
    ),
    Triple(
        "Publish the app",
        "https://console.cloud.google.com/auth/audience",
        "Under Publishing status tap Publish app, then Confirm; skip the verification prompt. While it says Testing, Google ends the connection every 7 days.",
    ),
)

private val troubleshooting = listOf(
    "Error 403: access_denied" to "Your college email isn't a test user (step 4), or you picked the wrong Google account.",
    "\"Use your @vitapstudent.ac.in Google account\"" to "You chose a personal account in Google's picker. Try again and choose your college account.",
    "\"Create a Desktop app OAuth client\"" to "The JSON is from a Web or Android client. Redo step 5 and choose Desktop app.",
    "\"Gmail didn't let the app in\"" to "The Gmail API isn't on in this project (step 2). It can take a minute to apply.",
    "Worked, then stopped after a week" to "The project is still in Testing. Do step 6, then connect again.",
    "Error 403: org_internal" to "The audience is Internal. Open Audience and switch it to External (step 3).",
    "Access blocked by your admin" to "VIT's Workspace policy blocks the app. Use mail forwarding instead.",
    "Using a friend's JSON?" to "It works and gives them no access to your Gmail, but they must add your college email as a test user unless their app is published.",
)

/**
 * Gmail in the OTP auto-fill sheet (the Flutter app's Gmail BYOK setup): off, the six Google
 * Cloud steps, Import JSON and Connect Google; connected, the account, when Google ends the grant
 * (weekly while the project is in Testing) with Reconnect, "Use this for OTPs", "Delete the OTP
 * mail after reading" and Disconnect. Flows `gmail.open/import/connect/disconnect`, lines [SRC]. Test tags
 * `gmail.*`.
 */
@Composable
internal fun GmailContent() {
    val account = Gmail.account
    var confirmOff by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { flow("gmail.open") { Log.info(SRC, "open", it, mapOf("connected" to "${account != null}")) } }
    if (account != null) Connected(account) { confirmOff = true } else Setup()
    if (confirmOff) {
        val scope = rememberCoroutineScope()
        AlertDialog(
            onDismissRequest = { confirmOff = false },
            title = { Text("Disconnect Gmail?") },
            text = { Text("OTPs are no longer read from your Gmail, and the app's access to it is revoked.") },
            confirmButton = {
                TextButton(
                    {
                        confirmOff = false
                        scope.launch { flow("gmail.disconnect") { Gmail.disconnect(it) } }
                    },
                    Modifier.testTag("gmail.off.confirm"),
                ) { Text("Disconnect", color = MaterialTheme.colorScheme.error) }
            },
            dismissButton = { TextButton({ confirmOff = false }) { Text("Cancel") } },
        )
    }
}

/** A Google sign-in in progress: [start] runs [Gmail.connect]; tapping again cancels. */
@Stable
private class Connecting(private val scope: CoroutineScope, private val open: (String) -> Unit) {
    var job by mutableStateOf<Job?>(null)
        private set
    var problem by mutableStateOf<String?>(null)

    fun toggle(client: Gmail.Client) {
        job?.let { it.cancel(); return }
        problem = null
        job = scope.launch {
            try {
                flow("gmail.connect") { id ->
                    // The college address ends with the registration number (not the VTOP username).
                    val regno = withContext(Dispatchers.IO) { AppCore.await().savedAccount(id)?.registrationNumber }.orEmpty()
                    Gmail.connect(client, regno, id, open)
                }
                showToast("Gmail connected", "VTOP's OTP now fills in from your Gmail.")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Gmail.GmailException) {
                Log.info(SRC, "connect failed: ${e.code}")
                problem = e.message
            } catch (e: Throwable) {
                Log.warn(SRC, "connect failed: ${e::class.simpleName}")
                problem = "Couldn't connect Gmail. Try again."
            } finally {
                job = null
            }
        }
    }
}

@Composable
private fun rememberConnecting(): Connecting {
    val scope = rememberCoroutineScope()
    val uri = LocalUriHandler.current
    return remember { Connecting(scope) { uri.openUri(it) } }
}

/** Connect (or Reconnect) Google; Cancel while the browser sign-in runs. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ConnectButton(connecting: Connecting, client: Gmail.Client?, label: String, modifier: Modifier) {
    val busy = connecting.job != null
    Button(
        if (busy || client != null) ({ client?.let(connecting::toggle) ?: connecting.job?.cancel() }) else null,
        modifier.testTag("gmail.connect"),
        variant = if (busy) ButtonVariant.Outline else ButtonVariant.Primary,
        prefix = if (busy) null else Lucide.LogIn,
    ) {
        if (busy) LoadingIndicator(Modifier.size(20.dp))
        Text(if (busy) "Cancel" else label)
    }
}

/** Android freezes the app behind the browser, so Google's answer waits until the student is back. */
@Composable
private fun WaitingHint() = Callout(
    Lucide.LogIn,
    AppTheme.tones.accentTone,
    "Signing in with Google… After Continue on Google's page, come back here; it finishes as soon as you do.",
)

@Composable
private fun Connected(account: Gmail.Account, onOff: () -> Unit) {
    val scheme = MaterialTheme.colorScheme
    val tones = AppTheme.tones
    val connecting = rememberConnecting()
    val days = account.daysLeft
    when {
        account.needsReconnect -> Callout(Lucide.TriangleAlert, tones.warning, "Google ended the connection for ${account.email}, as it does every 7 days while the Google project is in Testing. Reconnect: same JSON, one sign-in.")
        else -> Callout(Lucide.MailCheck, tones.success, "VTOP's OTP fills in from ${account.email}.") {
            Text("Client ${account.client.shortId} · ${account.client.project}", style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
        }
    }
    if (account.needsReconnect || (days != null && days <= 2)) {
        ConnectButton(connecting, account.client, "Reconnect Google", Modifier.fillMaxWidth())
    }
    connecting.problem?.let { Callout(Lucide.TriangleAlert, tones.danger, it) }
    if (connecting.job != null) WaitingHint()
    if (days != null && !account.needsReconnect) {
        Text(
            "Google ends this connection in ${if (days == 0L) "less than a day" else "$days d"}: the project is in Testing. Publish it (step 6 in the setup) and reconnect to stop the weekly sign-in.",
            Modifier.testTag("gmail.grant"),
            style = MaterialTheme.typography.bodySmall,
            color = scheme.onSurfaceVariant,
        )
    }
    Row(verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Text("Delete the OTP mail after reading", style = MaterialTheme.typography.bodyLarge)
            Text("Off: it's only marked read", style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
        }
        Switch(account.deleteAfterReading, Gmail::setDeleteAfterReading, Modifier.testTag("gmail.delete_after"))
    }
    Button(onOff, Modifier.fillMaxWidth().testTag("gmail.off"), variant = ButtonVariant.Outline) { Text("Disconnect", color = scheme.error) }
}

@Composable
private fun Setup() {
    val scheme = MaterialTheme.colorScheme
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val connecting = rememberConnecting()
    var client by remember { mutableStateOf<Gmail.Client?>(null) }
    var help by remember { mutableStateOf(false) }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { picked: Uri? ->
        picked ?: return@rememberLauncherForActivityResult
        scope.launch {
            flow("gmail.import") { id ->
                try {
                    val bytes = withContext(Dispatchers.IO) { context.contentResolver.openInputStream(picked)?.use { it.readBytes() } } ?: ByteArray(0)
                    client = Gmail.parseClient(bytes)
                    connecting.problem = null
                    Log.info(SRC, "client imported", id)
                } catch (e: Gmail.GmailException) {
                    Log.info(SRC, "client refused: ${e.code}", id)
                    connecting.problem = e.message
                }
            }
        }
    }
    Text(
        "Reads VTOP's OTP mail straight from your college Gmail through your own Google Cloud project, so no one else holds access. About 10 minutes, once; a computer makes steps 1–6 easier.",
        style = MaterialTheme.typography.bodyMedium,
        color = scheme.onSurfaceVariant,
    )
    steps.forEachIndexed { i, (title, link, body) -> Step(i + 1, title, link, "Open in Google Cloud", tag = "gmail") { body } }
    Step(7, "Connect", null, null, tag = "gmail", last = true) {
        "Import the JSON, tap Connect Google and choose your @vitapstudent.ac.in account. On \"Google hasn't verified this app\", check the name is yours, then Continue. Tick the Gmail permission, tap Continue, then come back to Vitap Mate: Android pauses the app while you're in the browser, so Google's page keeps loading until you do."
    }
    connecting.problem?.let { Callout(Lucide.TriangleAlert, AppTheme.tones.danger, it) }
    if (connecting.job != null) WaitingHint()
    client?.let { Text("${it.project} · ${it.shortId}", Modifier.testTag("gmail.client"), style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant) }
    Row(horizontalArrangement = Arrangement.spacedBy(Space.sm), verticalAlignment = Alignment.CenterVertically) {
        Button(
            if (connecting.job == null) ({ picker.launch(arrayOf("application/json", "application/octet-stream", "text/plain")) }) else null,
            Modifier.weight(1f).testTag("gmail.import"),
            variant = if (client == null) ButtonVariant.Primary else ButtonVariant.Outline,
            prefix = Lucide.FileBraces,
        ) { Text(if (client == null) "Import JSON" else "Change JSON") }
        ConnectButton(connecting, client, "Connect Google", Modifier.weight(1f))
    }
    Row(Modifier.fillMaxWidth().clickable { help = !help }.testTag("gmail.help"), verticalAlignment = Alignment.CenterVertically) {
        Text("Something went wrong?", Modifier.weight(1f), style = MaterialTheme.typography.titleSmall)
        Icon(if (help) Lucide.ChevronUp else Lucide.ChevronDown, null, Modifier.size(18.dp))
    }
    if (help) {
        troubleshooting.forEach { (title, fix) ->
            Column {
                Text(title, style = MaterialTheme.typography.bodyMedium)
                Text(fix, style = MaterialTheme.typography.bodySmall, color = scheme.onSurfaceVariant)
            }
        }
    }
}
