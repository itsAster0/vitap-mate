package vitapmate

import android.Manifest
import android.content.Context
import android.content.Intent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalContext
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Radio
import kotlinx.coroutines.launch
import vitapmate.data.Foreground
import vitapmate.data.Settings
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.rememberHaptics

/**
 * Settings → Live updates (Android 16+ only; nothing on older phones): off by default. Turning it on
 * asks for the notification permission; while the system doesn't promote the app's notifications,
 * a row under it opens that system setting.
 */
@Composable
fun LiveUpdatesSetting() {
    if (!LiveUpdates.supported()) return
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        scope.logAction("settings.notifications", "ui.settings", "notification permission", mapOf("granted" to "$granted"))
        scope.launch { LiveUpdates.refresh(context, "live.settings"); LiveUpdates.outings(context, "live.settings") }
    }
    val set = { on: Boolean ->
        Settings.liveUpdates = on
        haptics.toggle(on)
        if (on) ask.launch(Manifest.permission.POST_NOTIFICATIONS) else scope.launch { LiveUpdates.refresh(context, "live.settings") }
        scope.logAction("settings.live_updates", "ui.settings", "live updates ${if (on) "on" else "off"}")
    }
    // Read again on return from the system setting.
    val promotable = remember(Foreground.resumes) { LiveUpdates.promotable(context) }
    ListGroup("Live updates", footer = "On the lock screen and in the status bar, from what the app has saved.") {
        GroupItem(
            "Class and outing",
            "settings.live_updates",
            Lucide.Radio,
            "The class on now and the next one; an outing request until VTOP decides",
            onClick = { set(!Settings.liveUpdates) },
            trailing = { ItemSwitch(Settings.liveUpdates, { set(it) }) },
            on = Settings.liveUpdates,
        )
        if (Settings.liveUpdates && !promotable) GroupItem(
            "Allow Live updates",
            "settings.live_updates.allow",
            Lucide.ExternalLink,
            "Android shows them as plain notifications until you allow it",
            onClick = { allowLiveUpdates(context) },
            tone = AppTheme.tones.warning,
        )
    }
}

/** Live updates on, but Android doesn't promote them yet (the "Allow Live updates" row). */
fun liveUpdatesBlocked(context: Context): Boolean = LiveUpdates.supported() && Settings.liveUpdates && !LiveUpdates.promotable(context)

/** Opens Android's setting that lets the app's notifications show as Live updates. */
fun allowLiveUpdates(context: Context) = context.startActivity(
    Intent(android.provider.Settings.ACTION_APP_NOTIFICATION_PROMOTION_SETTINGS)
        .putExtra(android.provider.Settings.EXTRA_APP_PACKAGE, context.packageName),
)
