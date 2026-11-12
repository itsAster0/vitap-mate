package vitapmate

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SuggestionChip
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.Bug
import com.composables.icons.lucide.CalendarMinus
import com.composables.icons.lucide.ChevronRight
import com.composables.icons.lucide.FolderDown
import com.composables.icons.lucide.History
import com.composables.icons.lucide.Key
import com.composables.icons.lucide.LogOut
import com.composables.icons.lucide.Network
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Unplug
import com.composables.icons.lucide.RefreshCcw
import com.composables.icons.lucide.Server
import com.composables.icons.lucide.TriangleAlert
import com.composables.icons.lucide.Utensils
import kotlinx.coroutines.launch
import vitapmate.core.Core
import vitapmate.core.Route as VtopRoute
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import vitapmate.data.AccountBackup
import vitapmate.data.AccountState
import vitapmate.data.Downloads
import vitapmate.data.Foreground
import vitapmate.data.OtpPrompt
import vitapmate.data.PageFetches
import vitapmate.data.Settings
import vitapmate.data.coreFlow
import vitapmate.data.orNull
import vitapmate.features.attendance.ATTENDANCE_GUIDE
import vitapmate.features.attendance.SKIP_DISCLAIMER
import vitapmate.features.docs.MessTimingsSheet
import vitapmate.features.docs.messTimingsSummary
import vitapmate.features.login.OtpAutofillSheet
import vitapmate.observability.Log
import vitapmate.ui.AppTheme
import vitapmate.ui.AvatarPhoto
import vitapmate.ui.DownloadFolderDialog
import vitapmate.ui.StatusChip
import vitapmate.ui.neutralTone
import vitapmate.ui.rememberHaptics

/** Account (with the semester), appearance (theme, colours, tabs), diagnostics and about. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun SettingsScreen(onSignedOut: () -> Unit) {
    val nav = LocalNav.current
    val scope = rememberCoroutineScope()
    val deviceAuth = rememberDeviceAuth()
    val haptics = rememberHaptics()
    val user by produceState<String?>(null, accountVersion) {
        value = orNull { coreFlow("settings.user", Core::signedInUser) }
    }
    // The name, VTOP's photo and the registration number from the cached profile, when the Profile page has fetched it.
    val profile by produceState<Triple<String, String, String>?>(null) {
        value = orNull { coreFlow("settings.profile", Core::cachedProfile) }
            ?.let { p -> p.name.ifBlank { null }?.let { Triple(it, p.photo, p.registrationNumber) } }
    }
    // The chosen route, for the Connection row's chip (the same read the Connection page makes).
    val route by produceState<VtopRoute?>(null) {
        value = orNull { coreFlow("settings.route") { c, f -> c.routeStatus(f) } }?.chosen
    }
    val context = LocalContext.current
    val list = rememberLazyListState()
    var otpSheet by remember { mutableStateOf(false) }
    // What already shows as a warning on a row, repeated at the top; each does what its row does.
    val needs = buildList {
        if (AccountState.rejected) add(Need("Username and password", "VTOP rejected the saved password", "account") { deviceAuth.confirm("Account") { nav.push(Route.Screen(ACCOUNT)) } })
        syncPaused()?.let { add(Need("Background sync", "Paused: $it", "sync") { nav.push(Route.Screen(SYNC_HISTORY)) }) }
        otpWarning()?.let { add(Need("OTP auto-fill", it, "otp") { otpSheet = true }) }
        val resumes = Foreground.resumes
        if (remember(resumes) { ClassMode.blocked(context) }) add(Need("Allow Do Not Disturb access", "Nothing is silenced until you allow vitapmate", "class_mode") { ClassMode.allow(context) })
        if (remember(resumes) { liveUpdatesBlocked(context) }) add(Need("Allow Live updates", "Android shows them as plain notifications until you allow it", "live_updates") { allowLiveUpdates(context) })
    }
    // Each group's title and its item's index in the list below (the profile, "Needs you" and these chips come first).
    val jumps = listOfNotNull(
        "Account" to 3, "Sync" to 4, "Connections" to 5, "Attendance" to 6,
        ("Live updates" to 7).takeIf { LiveUpdates.supported() },
        ("In class" to 8).takeIf { ClassMode.supported() },
        ("Assistant" to 9).takeIf { android.os.Build.VERSION.SDK_INT >= 36 },
        "Appearance" to 10, "Files and mess" to 11, "Diagnostics" to 12,
    )
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
        LazyColumn(state = list, contentPadding = padding) {
            item { ProfileHeader(user ?: LocalUser.current, profile?.first, profile?.second, profile?.third) }
            // Always an item (empty when nothing is wrong), so the groups keep their indices for the chips.
            item { if (needs.isNotEmpty()) NeedsYou(needs) }
            item {
                LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    items(jumps) { (title, index) ->
                        SuggestionChip(
                            { scope.launch { list.animateScrollToItem(index) } },
                            { Text(title) },
                            Modifier.testTag("settings.jump.${title.lowercase().replace(' ', '_')}"),
                        )
                    }
                }
            }
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
                        trailing = { ItemSwitch(Settings.autoRefresh, { Settings.autoRefresh = it; haptics.toggle(it) }) },
                        on = Settings.autoRefresh,
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
                        below = route?.let { { StatusChip(it.title(), neutralTone, Modifier.padding(top = 6.dp)) } },
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
                        trailing = { ItemSwitch(Settings.showSkipAdvice, { setSkip(it) }) },
                        on = Settings.showSkipAdvice,
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
                val danger = AppTheme.tones.danger
                Button(
                    { confirmSignOut = true },
                    Modifier.fillMaxWidth().padding(start = 18.dp, top = 26.dp, end = 18.dp, bottom = 12.dp).height(58.dp).testTag("settings.sign_out"),
                    shape = CircleShape,
                    colors = ButtonDefaults.buttonColors(containerColor = danger.subtle, contentColor = danger.onSubtle),
                ) {
                    Icon(Lucide.LogOut, null, Modifier.size(20.dp), tint = danger.base)
                    Spacer(Modifier.width(10.dp))
                    Text("Sign out", style = MaterialTheme.typography.titleSmallEmphasized)
                }
            }
        }
    }

    if (otpSheet) OtpAutofillSheet { otpSheet = false }
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

/** A problem in "Needs you": a row's [title] and warning [text]; [onClick] does what tapping that row does. */
private class Need(val title: String, val text: String, val tag: String, val onClick: () -> Unit)

/** Live problems from the rows below, in the warning tone, under the profile header. */
@Composable
private fun NeedsYou(needs: List<Need>) {
    val warning = AppTheme.tones.warning
    Column(
        Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp).clip(MaterialTheme.shapes.extraLarge).background(warning.subtle)
            .padding(top = 14.dp, bottom = 6.dp).testTag("settings.needs_you"),
    ) {
        Text("Needs you · ${needs.size}", Modifier.padding(horizontal = 18.dp), style = MaterialTheme.typography.labelLarge, color = warning.base)
        needs.forEach { need ->
            Row(
                Modifier.fillMaxWidth().clickable(onClick = need.onClick).testTag("settings.needs_you.${need.tag}").padding(horizontal = 18.dp, vertical = 10.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Icon(Lucide.TriangleAlert, null, Modifier.size(20.dp), tint = warning.base)
                Column(Modifier.weight(1f)) {
                    Text(need.title, style = MaterialTheme.typography.titleSmall, color = warning.onSubtle)
                    Text(need.text, style = MaterialTheme.typography.bodyMedium, color = warning.onSubtle)
                }
                Icon(Lucide.ChevronRight, null, Modifier.size(20.dp), tint = warning.onSubtle)
            }
        }
    }
}
