package vitapmate

import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.Bug
import com.composables.icons.lucide.CalendarMinus
import com.composables.icons.lucide.FolderDown
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Key
import com.composables.icons.lucide.LogOut
import com.composables.icons.lucide.Network
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Unplug
import com.composables.icons.lucide.RefreshCcw
import com.composables.icons.lucide.Server
import com.composables.icons.lucide.Utensils
import kotlinx.coroutines.launch
import vitapmate.core.Core
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.data.AccountBackup
import vitapmate.data.AccountState
import vitapmate.data.Downloads
import vitapmate.data.OtpPrompt
import vitapmate.data.PageFetches
import vitapmate.data.Settings
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.features.attendance.ATTENDANCE_GUIDE
import vitapmate.features.attendance.SKIP_DISCLAIMER
import vitapmate.features.docs.MessTimingsSheet
import vitapmate.features.docs.messTimingsSummary
import vitapmate.observability.Log
import vitapmate.ui.AppTheme
import vitapmate.ui.AvatarPhoto
import vitapmate.ui.DownloadFolderDialog
import vitapmate.ui.rememberHaptics

/** Account (with the semester), appearance (theme, colours, tabs), diagnostics and about. */
@Composable
fun SettingsScreen(onSignedOut: () -> Unit) {
    val nav = LocalNav.current
    val scope = rememberCoroutineScope()
    val deviceAuth = rememberDeviceAuth()
    val haptics = rememberHaptics()
    val user by produceState<String?>(null, accountVersion) {
        value = orNull { coreFlow("settings.user", Core::signedInUser) }
    }
    // The name and VTOP's photo from the cached profile, when the Profile page has fetched it.
    val profile by produceState<Pair<String, String>?>(null) {
        value = orNull { coreFlow("settings.profile", Core::cachedProfile) }
            ?.let { it.name.ifBlank { null }?.to(it.photo) }
    }
    var confirmSignOut by remember { mutableStateOf(false) }
    var messTimings by remember { mutableStateOf(false) }
    var downloadFolder by remember { mutableStateOf(false) }
    var skipDisclaimer by remember { mutableStateOf(false) }
    // Turning skip advice on asks first (a disclaimer); turning it off doesn't.
    val setSkip = { on: Boolean ->
        if (on) {
            skipDisclaimer = true
        } else {
            Settings.showSkipAdvice = false
            haptics.toggle(false)
        }
    }

    Page("Settings", SETTINGS) { padding ->
        LazyColumn(contentPadding = padding) {
            item { ProfileHeader(user ?: LocalUser.current, profile?.first, profile?.second) }
            item {
                ListGroup("Account") {
                    GroupItem(
                        "Username and password",
                        "settings.account",
                        Lucide.Key,
                        if (AccountState.rejected) "VTOP rejected the saved password" else null,
                        onClick = { deviceAuth.confirm("Account") { nav.push(Route.Screen(ACCOUNT)) } },
                        trailing = { Chevron() },
                        tone = if (AccountState.rejected) AppTheme.tones.warning else null,
                    )
                    SemesterSetting()
                    OtpAutofillSetting()
                }
            }
            item {
                ListGroup("Sync") {
                    GroupItem(
                        "Auto refresh",
                        "settings.auto_refresh",
                        Lucide.RefreshCcw,
                        "Fetch pages on open, when old and when you come back",
                        onClick = { Settings.autoRefresh = !Settings.autoRefresh; haptics.toggle(Settings.autoRefresh) },
                        trailing = { Switch(Settings.autoRefresh, { Settings.autoRefresh = it; haptics.toggle(it) }) },
                    )
                    BackgroundSyncRow { nav.push(Route.Screen(SYNC_HISTORY)) }
                }
            }
            item {
                ListGroup("Connections", footer = "How the app reaches VTOP, and what it shares with the vitapmate server.") {
                    GroupItem(
                        "Connection",
                        "settings.connection",
                        Lucide.Network,
                        "How the app reaches VTOP",
                        onClick = { nav.push(Route.Screen(CONNECTION)) },
                        trailing = { Chevron() },
                    )
                    GroupItem(
                        "Connected apps",
                        "settings.connected_apps",
                        Lucide.Unplug,
                        "Browser extension and AI apps (MCP)",
                        onClick = { nav.push(Route.Screen(CONNECTED_APPS)) },
                        trailing = { Chevron() },
                    )
                    GroupItem(
                        "Server",
                        "settings.server",
                        Lucide.Server,
                        "What the server keeps and your linked phones",
                        onClick = { nav.push(Route.Screen(SERVER)) },
                        trailing = { Chevron() },
                    )
                }
            }
            item {
                ListGroup("Attendance") {
                    GroupItem(
                        "Classes you can skip",
                        "settings.skip_advice",
                        Lucide.CalendarMinus,
                        "On attendance and the timetable",
                        onClick = { setSkip(!Settings.showSkipAdvice) },
                        trailing = { Switch(Settings.showSkipAdvice, { setSkip(it) }) },
                    )
                    GroupItem(
                        "How the numbers work",
                        "settings.attendance_guide",
                        Lucide.BookOpen,
                        "What each number means and when it can be off",
                        onClick = { nav.push(Route.Detail("attendance", ATTENDANCE_GUIDE, "How the numbers work")) },
                        trailing = { Chevron() },
                    )
                }
            }
            item { LiveUpdatesSetting() }
            item { ClassModeSetting() }
            item { AssistantSetting() }
            item { AppearanceGroup() }
            item {
                ListGroup("Files and mess") {
                    GroupItem("Download folder", "settings.download_folder", Lucide.FolderDown, Downloads.label(), onClick = { downloadFolder = true }, trailing = { Chevron() })
                    GroupItem("Mess timings", "settings.mess_timings", Lucide.Utensils, messTimingsSummary(), onClick = { messTimings = true }, trailing = { Chevron() })
                }
            }
            item {
                ListGroup("Diagnostics") {
                    GroupItem(
                        "Activity",
                        "settings.activity",
                        Lucide.History,
                        "What the app fetched from VTOP",
                        onClick = { nav.push(Route.Screen(ACTIVITY_HISTORY)) },
                        trailing = { Chevron() },
                    )
                    if (Settings.developerEnabled) {
                        GroupItem(
                            "Developer",
                            "settings.developer",
                            Lucide.Bug,
                            "Session, cache, refresh times and tools",
                            onClick = { nav.push(Route.Screen(DEVELOPER)) },
                            trailing = { Chevron() },
                            tone = AppTheme.tones.warning,
                        )
                    }
                }
            }
            item {
                ListGroup(null) {
                    GroupItem("Sign out", "settings.sign_out", Lucide.LogOut, onClick = { confirmSignOut = true }, tone = AppTheme.tones.danger)
                }
            }
        }
    }

    if (messTimings) MessTimingsSheet { messTimings = false }
    if (downloadFolder) DownloadFolderDialog({ downloadFolder = false })

    if (skipDisclaimer) {
        AlertDialog(
            onDismissRequest = { skipDisclaimer = false },
            title = { Text("Classes you can skip") },
            text = { Text(SKIP_DISCLAIMER) },
            confirmButton = {
                TextButton(
                    {
                        skipDisclaimer = false
                        Settings.showSkipAdvice = true
                        haptics.toggle(true)
                        Log.info("ui.settings", "skip advice on (disclaimer accepted)")
                    },
                    Modifier.testTag("settings.skip_disclaimer.on"),
                ) { Text("Turn on") }
            },
            dismissButton = {
                TextButton({ skipDisclaimer = false }, Modifier.testTag("settings.skip_disclaimer.cancel")) { Text("Cancel") }
            },
        )
    }

    if (confirmSignOut) {
        val context = androidx.compose.ui.platform.LocalContext.current
        AlertDialog(
            onDismissRequest = { confirmSignOut = false },
            title = { Text("Sign out?") },
            text = { Text("The saved account and cached pages are removed from this phone.") },
            confirmButton = {
                TextButton(
                    {
                        confirmSignOut = false
                        scope.launch {
                            runCatching { coreFlow("sign_out", Core::signOut) }
                                .onFailure { Log.error("ui.settings", "sign out failed: $it") }
                            withContext(Dispatchers.IO) { AccountBackup.forgetAccount(context) }
                            // A collapsed OTP prompt and running page fetches belong to the session
                            // just dropped: not over the login screen, not into the next account.
                            OtpPrompt.cancel()
                            PageFetches.cancelAll()
                            // The photo belongs to this account.
                            AvatarPhoto.remove()
                            // The VTOP web view's cookies are this session's.
                            runCatching { android.webkit.CookieManager.getInstance().removeAllCookies(null) }
                            // Its recently opened VTOP pages too (the Flutter key).
                            context.getSharedPreferences("settings", android.content.Context.MODE_PRIVATE).edit().remove("settings_vtop_recent_pages").apply()
                            onSignedOut()
                        }
                    },
                    Modifier.testTag("settings.sign_out.confirm"),
                ) { Text("Sign out") }
            },
            dismissButton = { TextButton({ confirmSignOut = false }) { Text("Cancel") } },
        )
    }
}
