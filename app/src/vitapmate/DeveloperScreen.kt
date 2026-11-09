package vitapmate

import vitapmate.ui.rememberHaptics
import android.content.ClipData
import android.content.ClipboardManager
import android.content.Intent
import android.content.pm.ApplicationInfo
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.CalendarRange
import com.composables.icons.lucide.CalendarX
import com.composables.icons.lucide.CircleMinus
import com.composables.icons.lucide.ClipboardList
import com.composables.icons.lucide.Clock
import com.composables.icons.lucide.Cookie
import com.composables.icons.lucide.Database
import com.composables.icons.lucide.Hourglass
import com.composables.icons.lucide.KeySquare
import com.composables.icons.lucide.BellRing
import com.composables.icons.lucide.Bug
import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Radio
import com.composables.icons.lucide.RefreshCw
import com.composables.icons.lucide.Mail
import com.composables.icons.lucide.Pin
import com.composables.icons.lucide.RotateCw
import com.composables.icons.lucide.Scan
import com.composables.icons.lucide.Server
import com.composables.icons.lucide.Share2
import com.composables.icons.lucide.SquareActivity
import com.composables.icons.lucide.Timer
import com.composables.icons.lucide.Trash
import com.composables.icons.lucide.Unplug
import com.composables.icons.lucide.Zap
import kotlin.coroutines.cancellation.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.CacheScope
import vitapmate.core.Core
import vitapmate.data.AppCore
import vitapmate.data.OtpPrompt
import vitapmate.data.PageFetches
import vitapmate.data.SemesterChoice
import vitapmate.data.Settings
import vitapmate.data.Tunable
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.AppTheme
import vitapmate.ui.PasswordField
import vitapmate.ui.ToastTone
import vitapmate.ui.showToast

/** Route id of Settings → Developer (not a tab choice). */
const val DEVELOPER = "developer"


/** A destructive action waiting for its confirmation. */
private class Confirm(val title: String, val body: String, val action: String, val run: suspend () -> Unit)

/**
 * Settings → Developer, the Flutter app's Debug page: logs and the Inspector, the session, refresh
 * times, saved data and mail forwarding. Actions are flows `dev.*` (log src `ui.dev`); clearing
 * never signs out: the saved account signs in again on the next fetch.
 */
@Composable
fun DeveloperScreen() {
    val nav = LocalNav.current
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var confirm by remember { mutableStateOf<Confirm?>(null) }
    var editing by remember { mutableStateOf<Tunable?>(null) }
    var editServer by remember { mutableStateOf(false) }
    var setAccount by remember { mutableStateOf(false) }
    val haptics = rememberHaptics()
    var serverRead by remember { mutableIntStateOf(0) }
    val server by produceState<String?>(null, serverRead) { value = dev("dev.server") { c, f -> c.devForwardingServer(f) } }
    val debuggable = remember { context.applicationInfo.flags and ApplicationInfo.FLAG_DEBUGGABLE != 0 }

    /** Runs [work] in flow [name]; [done] is the toast. A failure is logged and toasted. */
    fun act(name: String, done: (String) -> Pair<String, String>?, work: suspend (Core, String) -> String) = scope.launch {
        dev(name, work)?.let { result -> done(result)?.let { (title, text) -> showToast(title, text) } }
            ?: showToast("Didn't work", "See flow $name in the Inspector.", tone = ToastTone.Error)
    }

    @Composable
    fun tunable(t: Tunable, icon: ImageVector) = GroupItem(
        t.title,
        "dev.${t.key.removePrefix("dev_")}",
        icon,
        t.note,
        onClick = { editing = t },
        trailing = { TrailingValue(if (t == Tunable.SessionReuse && Settings.tunable(t) == 0) "Any age" else "${Settings.tunable(t)} ${t.unit}") },
    )

    Page("Developer", DEVELOPER) { padding ->
        LazyColumn(contentPadding = padding) {
            item {
                // As on Android: on from tapping the version 7 times, off here (and Developer leaves Settings).
                ListGroup(null) {
                    GroupItem(
                        "Developer options",
                        "dev.enabled",
                        Lucide.Bug,
                        onClick = { Settings.developerEnabled = false; haptics.toggle(false); nav.pop() },
                        trailing = { Switch(true, { if (!it) { Settings.developerEnabled = false; haptics.toggle(false); nav.pop() } }) },
                        tone = AppTheme.tones.warning,
                    )
                }
            }
            item {
                ListGroup("What happened") {
                    GroupItem("Inspector", "dev.inspector", Lucide.SquareActivity, "Flows with their Kotlin and Rust lines", onClick = { nav.push(Route.Screen(INSPECTOR)) }, trailing = { Chevron() })
                    GroupItem("Share logs", "dev.share_logs", Lucide.Share2, "The log file, through the share sheet", onClick = { scope.launch { shareLogs(context) } })
                    if (debuggable) {
                        GroupItem("Write state dump", "dev.state_dump", Lucide.ClipboardList, "files/state.json, as the vitapmate.DUMP broadcast", onClick = {
                            context.sendBroadcast(Intent("vitapmate.DUMP").setPackage(context.packageName))
                            showToast("State dumped", "files/state.json")
                        })
                    }
                    GroupItem(
                        "Inspect traffic",
                        "dev.inspect_traffic",
                        Lucide.Scan,
                        "Wi-Fi proxy and user CAs, from the next launch",
                        onClick = { Settings.inspectTraffic = !Settings.inspectTraffic; haptics.toggle(Settings.inspectTraffic) },
                        trailing = { Switch(Settings.inspectTraffic, { Settings.inspectTraffic = it; haptics.toggle(it) }) },
                    )
                    if (android.os.Build.VERSION.SDK_INT >= 34) GroupItem(
                        "HttpEngine",
                        "dev.http_engine",
                        Lucide.Zap,
                        "Android 14+, HTTP/3; from the next launch",
                        onClick = { Settings.httpEngine = !Settings.httpEngine; haptics.toggle(Settings.httpEngine) },
                        trailing = { Switch(Settings.httpEngine, { Settings.httpEngine = it; haptics.toggle(it) }) },
                    )
                }
            }
            item {
                ListGroup("Session") {
                    GroupItem("Set credentials without checking", "dev.set_account", Lucide.KeySquare, "Replace the saved ones; VTOP isn't asked", onClick = { setAccount = true })
                    tunable(Tunable.SessionReuse, Lucide.Timer)
                    tunable(Tunable.VtopTimeout, Lucide.Hourglass)
                    tunable(Tunable.SessionAnswer, Lucide.Hourglass)
                    GroupItem("Copy saved cookies", "dev.copy_cookies", Lucide.Cookie, "Cookie-Editor JSON, for a browser", onClick = {
                        scope.launch {
                            val json = dev("dev.copy_cookies") { c, f -> c.devCookieEditorJson(f).orEmpty() }
                            when {
                                json == null -> showToast("Didn't work", "See flow dev.copy_cookies in the Inspector.", tone = ToastTone.Error)
                                json.isEmpty() -> showToast("No saved cookies", "Refresh a page once and try again.")
                                else -> {
                                    context.getSystemService(ClipboardManager::class.java)?.setPrimaryClip(ClipData.newPlainText("VTOP cookies", json))
                                    showToast("Copied", "Cookie-Editor JSON is on the clipboard.")
                                }
                            }
                        }
                    })
                    GroupItem("Clear session", "dev.clear_session", Lucide.Unplug, "Drop VTOP's cookies; the next fetch signs in again", onClick = {
                        confirm = Confirm("Clear the session?", "The saved VTOP session is dropped. Your account stays; the next fetch signs in again, and VTOP may ask for an OTP.", "Clear") {
                            if (OtpPrompt.message != null) OtpPrompt.cancel()
                            dev("dev.clear_session") { c, f -> c.devClearSession(f) }?.let {
                                // The web view's cookies are the same session's.
                                runCatching { android.webkit.CookieManager.getInstance().removeAllCookies(null) }
                                showToast("Session cleared", "The next fetch signs in again.")
                            } ?: showToast("Didn't work", "See flow dev.clear_session in the Inspector.", tone = ToastTone.Error)
                        }
                    })
                }
            }
            item {
                ListGroup("Refresh", footer = "Auto refresh and background sync are in Settings → Sync.") {
                    GroupItem(
                        "Show background errors",
                        "dev.background_errors",
                        Lucide.BellRing,
                        "Toasts when automatic refreshes fail",
                        onClick = { Settings.showBackgroundErrors = !Settings.showBackgroundErrors; haptics.toggle(Settings.showBackgroundErrors) },
                        trailing = { Switch(Settings.showBackgroundErrors, { Settings.showBackgroundErrors = it; haptics.toggle(it) }) },
                    )
                    GroupItem(
                        "Run background sync now",
                        "dev.sync_now",
                        Lucide.RefreshCw,
                        if (Settings.backgroundSync) "Once, in the background, as a scheduled run would" else "Turn on background sync in Settings first",
                        enabled = Settings.backgroundSync,
                        onClick = {
                            BackgroundSync.runNow(context)
                            showToast("Background sync started", "Go to the home screen; a notification shows if anything changed.")
                        },
                    )
                    GroupItem(
                        "Show a test notification",
                        "dev.test_notification",
                        Lucide.BellRing,
                        "A sample change, the way background sync posts them",
                        onClick = { BackgroundSync.notifyTest(context) },
                    )
                    if (LiveUpdates.supported()) GroupItem(
                        "Preview live updates",
                        "dev.live_preview",
                        Lucide.Radio,
                        if (Settings.liveUpdates) "A sample class on now and an approved outing" else "Turn on Live updates in Settings first",
                        enabled = Settings.liveUpdates,
                        onClick = { LiveUpdates.preview(context) },
                    )
                    tunable(Tunable.PageMaxAge, Lucide.Clock)
                    tunable(Tunable.JustFetched, Lucide.Clock)
                    tunable(Tunable.CalendarMaxAge, Lucide.Clock)
                    tunable(Tunable.SweepMaxAge, Lucide.Clock)
                    tunable(Tunable.SweepCooldown, Lucide.Clock)
                    tunable(Tunable.SweepQuick, Lucide.Clock)
                    tunable(Tunable.SlowPage, Lucide.Clock)
                    GroupItem("Reset times", "dev.reset_times", Lucide.RotateCw, "Every number on this page back to its default", onClick = {
                        Tunable.entries.forEach { Settings.setTunable(it, null) }
                        Log.info("ui.dev", "tunables reset")
                        showToast("Defaults back", "Timeout and session reuse apply from the next launch.")
                    })
                }
            }
            item {
                ListGroup("Saved data", footer = "Pages fetch again when they open. You stay signed in.") {
                    GroupItem("Clear this semester's cache", "dev.clear_semester", Lucide.Database, "Its pages, class histories and marks seen", onClick = {
                        confirm = Confirm("Clear this semester's cache?", "Every saved page of the semester shown is removed from this phone, with class histories and when marks were first seen.", "Clear") {
                            clearCache(CacheScope.SEMESTER, "This semester's pages fetch again.")
                        }
                    })
                    GroupItem("Clear all cached pages", "dev.clear_all", Lucide.Trash, "Every semester, the profile and grade history", onClick = {
                        confirm = Confirm("Clear all cached pages?", "Every saved page of every semester, the profile and the grade history are removed from this phone. The semester list and your pick stay.", "Clear") {
                            clearCache(CacheScope.ALL, "Pages fetch again when they open.")
                        }
                    })
                    GroupItem("Reset semester", "dev.reset_semester", Lucide.CalendarRange, "Ask which semester to show again", onClick = {
                        confirm = Confirm("Reset the semester?", "The app forgets which semester it shows and asks again. Saved pages stay.", "Reset") {
                            dev("dev.reset_semester") { c, f -> c.devResetSemester(f).also { SemesterChoice.readCache(f) } }?.let {
                                showToast("Semester reset", "Choose the semester to show.")
                            } ?: showToast("Didn't work", "See flow dev.reset_semester in the Inspector.", tone = ToastTone.Error)
                        }
                    })
                    GroupItem("Drop latest history entry", "dev.drop_history", Lucide.CalendarX, "Every course, saved until the next refresh", onClick = {
                        act("dev.drop_history", { "History trimmed" to "Dropped the latest entry from $it courses." }) { c, f -> "${c.devDropLatestHistory(f)}" }
                    })
                    GroupItem("Clear attendance", "dev.clear_attendance", Lucide.CircleMinus, "Show it as not posted, until the next refresh", onClick = {
                        act("dev.clear_attendance", { if (it == "true") "Attendance cleared" to "Refresh to bring it back." else "No attendance saved" to "Nothing to clear." }) { c, f ->
                            "${c.devClearAttendance(f)}"
                        }
                    })
                    GroupItem("Simulate VTOP changes", "dev.simulate_changes", Lucide.FlaskConical, "Roll back one class, one score, one slot and one exam; the next fetch finds them", onClick = {
                        act("dev.simulate_changes", { n -> "Rolled back $n pages" to "Fetch all (More) or a background run now records them in Changes." }) { c, f ->
                            "${c.devSimulateChanges(f)}"
                        }
                    })
                }
            }
            item {
                ListGroup("Mail forwarding") {
                    GroupItem("Forwarding server", "dev.forwarding_server", Lucide.Server, server ?: "…", onClick = { editServer = true })
                    tunable(Tunable.OtpPoll, Lucide.Mail)
                }
            }
        }
    }

    confirm?.let { c ->
        AlertDialog(
            onDismissRequest = { confirm = null },
            title = { Text(c.title) },
            text = { Text(c.body) },
            confirmButton = {
                TextButton({
                    confirm = null
                    scope.launch { c.run() }
                }, Modifier.testTag("dev.confirm")) { Text(c.action, color = AppTheme.tones.danger.base) }
            },
            dismissButton = { TextButton({ confirm = null }, Modifier.testTag("dev.cancel")) { Text("Cancel") } },
        )
    }
    editing?.let { t ->
        TextDialog(
            t.title,
            "${t.note}. Default ${t.default} ${t.unit}; ${t.range.first} to ${t.range.last}.",
            "${Settings.tunable(t)}",
            number = true,
            check = { it.toIntOrNull()?.takeIf { v -> v in t.range } != null },
            onDismiss = { editing = null },
            onDefault = {
                Settings.setTunable(t, null)
                Log.info("ui.dev", "tunable default", fields = mapOf("key" to t.key))
            },
        ) {
            Settings.setTunable(t, it.toInt())
            Log.info("ui.dev", "tunable set", fields = mapOf("key" to t.key, "value" to it))
        }
    }
    if (setAccount) CredentialsDialog({ setAccount = false }) { username, password ->
        scope.launch {
            dev("dev.set_account") { c, f -> c.devSetAccount(f, username, password) }?.let {
                accountVersion++
                haptics.confirm()
                showToast("Credentials set", "Not checked with VTOP.")
            } ?: showToast("Didn't work", "See flow dev.set_account in the Inspector.", tone = ToastTone.Error)
        }
    }
    if (editServer) {
        TextDialog(
            "Forwarding server",
            "New forwarding addresses are made on this server, e.g. http://127.0.0.1:8787 with adb reverse. An existing address keeps its own.",
            server.orEmpty(),
            number = false,
            check = { it.trim().startsWith("http://") || it.trim().startsWith("https://") },
            onDismiss = { editServer = false },
            onDefault = { act("dev.server", { serverRead++; "Default server" to "New setups use the default." }) { c, f -> c.devSetForwardingServer(f, null).let { "" } } },
        ) { url ->
            act("dev.server", { serverRead++; "Server set" to url.trim() }) { c, f -> c.devSetForwardingServer(f, url).let { "" } }
        }
    }
}

/** Clears cached pages: running fetches stop (they'd save again) and open pages read nothing, so they fetch. */
private suspend fun clearCache(scope: CacheScope, done: String) {
    PageFetches.cancelAll()
    dev("dev.clear_cache") { c, f -> c.devClearCache(f, scope).also { Log.info("ui.dev", "cache cleared", f, mapOf("scope" to scope.name)) } }
        ?.let { showToast("Cache cleared", done) }
        ?: showToast("Didn't work", "See flow dev.clear_cache in the Inspector.", tone = ToastTone.Error)
}

/** [work] on the core in flow [name], off the main thread; null when it failed (logged, never its data). */
private suspend fun <T : Any> dev(name: String, work: suspend (Core, String) -> T): T? = try {
    flow(name) { f -> withContext(Dispatchers.IO) { work(AppCore.await(), f) } }
} catch (e: CancellationException) {
    throw e
} catch (e: Exception) {
    Log.warn("ui.dev", "$name failed: ${e::class.simpleName}")
    null
}

/** Username and password for [Core.devSetAccount]; the password is never saved in instance state. */
@Composable
private fun CredentialsDialog(onDismiss: () -> Unit, onSave: (String, String) -> Unit) {
    var username by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    val ok = username.isNotBlank() && password.isNotEmpty()
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Set credentials") },
        text = {
            Column {
                OutlinedTextField(username, { username = it }, Modifier.testTag("dev.account.username"), label = { Text("Username") }, singleLine = true)
                PasswordField(password, { password = it }, "Password", "dev.account.password", Modifier.padding(top = 12.dp))
            }
        },
        confirmButton = {
            TextButton({
                onSave(username.trim(), password)
                onDismiss()
            }, Modifier.testTag("dev.account.save"), enabled = ok) { Text("Save") }
        },
        dismissButton = { TextButton(onDismiss) { Text("Cancel") } },
    )
}

/** An M3 dialog editing one value: Save (when [check] passes), Default, Cancel. */
@Composable
private fun TextDialog(
    title: String,
    note: String,
    current: String,
    number: Boolean,
    check: (String) -> Boolean,
    onDismiss: () -> Unit,
    onDefault: () -> Unit,
    onSave: (String) -> Unit,
) {
    var text by remember { mutableStateOf(current) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            Column {
                Text(note)
                OutlinedTextField(
                    text,
                    { text = it },
                    Modifier.padding(top = 12.dp).testTag("dev.edit.field"),
                    singleLine = true,
                    isError = !check(text),
                    keyboardOptions = KeyboardOptions(keyboardType = if (number) KeyboardType.Number else KeyboardType.Uri),
                )
            }
        },
        confirmButton = {
            TextButton({
                onSave(text.trim())
                onDismiss()
            }, Modifier.testTag("dev.edit.save"), enabled = check(text)) { Text("Save") }
        },
        dismissButton = {
            Row {
                TextButton({
                    onDefault()
                    onDismiss()
                }, Modifier.testTag("dev.edit.default")) { Text("Default") }
                TextButton(onDismiss) { Text("Cancel") }
            }
        },
    )
}
