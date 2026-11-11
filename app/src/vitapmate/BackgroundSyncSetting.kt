package vitapmate

import android.Manifest
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.material3.Icon
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import com.composables.icons.lucide.Check
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.RefreshCw
import kotlinx.coroutines.launch
import vitapmate.data.NOTIFY_KINDS
import vitapmate.data.SYNC_HOURS
import vitapmate.data.Settings
import vitapmate.features.login.MailForwarding
import vitapmate.observability.Log
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.ConnectedChoice

/**
 * Background sync's switch on its page ([SyncHistoryScreen]): off by default, greyed out until OTP auto-fill works (nobody
 * can type the code in the background), "Paused: …" in the warning tone after a run couldn't
 * finish the OTP. When on, one segment under it: how often it checks, and which changes notify. Turning it on asks for
 * the notification permission (Android 13+).
 */
/** Settings → Sync → Background sync: how it stands, and a way into its page. */
@Composable
fun BackgroundSyncRow(onClick: () -> Unit) {
    LaunchedEffect(Unit) { MailForwarding.load("settings") }
    val available = BackgroundSync.available()
    val on = Settings.backgroundSync && available
    val paused = Settings.syncPaused?.takeIf { on }
    GroupItem(
        "Background sync",
        "settings.background_sync",
        Lucide.RefreshCw,
        syncSummary(available, on, paused),
        onClick = onClick,
        trailing = { Chevron() },
        tone = if (paused != null) AppTheme.tones.warning else null,
    )
}

private fun syncSummary(available: Boolean, on: Boolean, paused: String?): String = when {
    !available -> "Needs OTP auto-fill: set it up in Settings → Account"
    paused != null -> "Paused: $paused"
    on -> "Checks VTOP every ${Settings.syncHours} h and tells you what changed"
    else -> "Check VTOP in the background and tell you what changed"
}

@Composable
fun BackgroundSyncSetting() {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    LaunchedEffect(Unit) { MailForwarding.load("settings") }
    val available = BackgroundSync.available()
    val on = Settings.backgroundSync && available
    val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        scope.logAction("settings.notifications", "ui.settings", "notification permission", mapOf("granted" to "$granted"))
    }
    val set = { value: Boolean ->
        Settings.backgroundSync = value
        if (value) Settings.syncPaused = null
        BackgroundSync.apply(context)
        if (value && Build.VERSION.SDK_INT >= 33) ask.launch(Manifest.permission.POST_NOTIFICATIONS)
        scope.logAction("settings.background_sync", "ui.settings", "background sync ${if (value) "on" else "off"}", mapOf("hours" to "${Settings.syncHours}"))
    }
    val paused = Settings.syncPaused?.takeIf { on }
    GroupItem(
        "Background sync",
        "sync.toggle",
        Lucide.RefreshCw,
        syncSummary(available, on, paused),
        enabled = available,
        onClick = { set(!on) },
        trailing = { ItemSwitch(on, { set(it) }, enabled = available) },
        on = on,
        tone = if (paused != null) AppTheme.tones.warning else null,
    )
    if (!on) return
    // Under the title (icon 40 dp + 12 dp gap): how often, and which changes notify.
    GroupSegment(Modifier.testTag("settings.sync_options")) {
        Column(Modifier.padding(start = 52.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Check every", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            ConnectedChoice(
                SYNC_HOURS,
                Settings.syncHours,
                { h ->
                    Settings.syncHours = h
                    BackgroundSync.apply(context)
                    scope.logAction("settings.sync_hours", "ui.settings", "sync every $h h", mapOf("hours" to "$h"))
                },
                tag = "settings.sync_hours",
                key = { "$it" },
                label = { "$it h" },
            )
            Text("Notify about", Modifier.padding(top = 8.dp), style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.onSurfaceVariant)
            FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                for (kind in NOTIFY_KINDS) {
                    val active = Settings.notifies(kind)
                    FilterChip(
                        active,
                        { Settings.setNotifies(kind, !active) },
                        { Text(kind.replaceFirstChar { it.uppercase() }, style = MaterialTheme.typography.labelLarge) },
                        Modifier.testTag("settings.notify.$kind"),
                        leadingIcon = if (active) {
                            { Icon(Lucide.Check, null, Modifier.size(16.dp)) }
                        } else null,
                    )
                }
            }
        }
    }
}
