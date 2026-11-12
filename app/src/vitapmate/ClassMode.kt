package vitapmate

import android.app.AlarmManager
import android.app.AutomaticZenRule
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.service.notification.Condition
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalContext
import com.composables.icons.lucide.BellOff
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Lucide
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.data.AppCore
import vitapmate.data.Foreground
import vitapmate.data.Settings
import vitapmate.features.timetable.istDay
import vitapmate.features.timetable.istMinute
import vitapmate.features.timetable.istSecond
import vitapmate.features.timetable.meeting
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.rememberHaptics

/**
 * Settings → In class (Android 10+, off by default): a Do Not Disturb rule of the app's own
 * ("In class", priority only) that is on while a class from the cached timetable is on. An
 * inexact alarm ([LiveUpdateReceiver]) flips it at each class's start and end, so it can be a few
 * minutes late; the student can change or remove the rule in Android's Modes. Flows `classmode.*`.
 */
object ClassMode {
    private val CONDITION: Uri = Uri.parse("condition://vitapmate/class")

    fun supported(): Boolean = Build.VERSION.SDK_INT >= 29

    /** Whether the student gave the app Do Not Disturb access. */
    fun granted(context: Context): Boolean = context.getSystemService(NotificationManager::class.java).isNotificationPolicyAccessGranted

    /** On, but without Do Not Disturb access (the "Allow Do Not Disturb access" row). */
    fun blocked(context: Context): Boolean = supported() && Settings.classMode && !granted(context)

    /** Opens Android's Do Not Disturb access setting. */
    fun allow(context: Context) = context.startActivity(Intent(android.provider.Settings.ACTION_NOTIFICATION_POLICY_ACCESS_SETTINGS))

    /** Brings the rule and its alarm in step with the setting and the cached timetable. */
    suspend fun refresh(context: Context, flowName: String) {
        if (!supported()) return
        flow(flowName) { f ->
            val app = context.applicationContext
            val manager = app.getSystemService(NotificationManager::class.java)
            val alarms = app.getSystemService(AlarmManager::class.java)
            val alarm = PendingIntent.getBroadcast(app, 1, Intent(app, LiveUpdateReceiver::class.java), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
            if (!granted(app)) return@flow alarms.cancel(alarm)
            val id = manager.automaticZenRules.entries.firstOrNull { it.value.conditionId == CONDITION }?.key
            if (!Settings.classMode) {
                alarms.cancel(alarm)
                if (id != null) manager.removeAutomaticZenRule(id)
                return@flow
            }
            val ms = System.currentTimeMillis()
            val today = istDay(ms)
            val now = istMinute(ms)
            // Days from today with their classes; null without a cached timetable.
            val days = runCatching {
                withContext(Dispatchers.IO) {
                    val c = AppCore.await()
                    c.cachedTimetable(f)?.let { c.timetableDays(f, today, 8).map { d -> d.day to meeting(d).map { it.slot }.sortedBy { it.start } } }
                }
            }.onFailure { Log.warn("classmode", "timetable unavailable: ${it::class.simpleName}", f) }.getOrNull()
            val todays = days?.firstOrNull { it.first == today }?.second.orEmpty()
            val current = todays.firstOrNull { it.start <= now && now < it.end }
            // Minutes from today's midnight to the next flip.
            val wake = current?.end
                ?: todays.firstOrNull { it.start > now }?.start
                ?: days?.firstOrNull { it.first > today && it.second.isNotEmpty() }?.let { (it.first - today) * 1440 + it.second.first().start }
            val rule = id ?: manager.addAutomaticZenRule(
                AutomaticZenRule("In class", null, ComponentName(app, MainActivity::class.java), CONDITION, null, NotificationManager.INTERRUPTION_FILTER_PRIORITY, true),
            )
            manager.setAutomaticZenRuleState(rule, Condition(CONDITION, current?.name ?: "No class now", if (current != null) Condition.STATE_TRUE else Condition.STATE_FALSE))
            if (wake != null) alarms.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, ms + (wake * 60L - istSecond(ms)) * 1000, alarm)
            Log.info("classmode", "rule", f, mapOf("on" to "${current != null}", "today" to "${todays.size}", "wake_in_min" to "${wake?.minus(now)}"))
        }
    }
}

/**
 * Settings → In class: the switch, and while Android hasn't given the app Do Not Disturb access a
 * row that opens that system setting (asked for when the switch is turned on).
 */
@Composable
fun ClassModeSetting() {
    if (!ClassMode.supported()) return
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val haptics = rememberHaptics()
    // Read again on return from the system setting, and the rule with it.
    val granted = remember(Foreground.resumes) { ClassMode.granted(context) }
    LaunchedEffect(Foreground.resumes) { ClassMode.refresh(context, "classmode.settings") }
    val allow = { ClassMode.allow(context) }
    val set = { on: Boolean ->
        Settings.classMode = on
        haptics.toggle(on)
        if (on && !granted) allow() else scope.launch { ClassMode.refresh(context, "classmode.settings") }
        scope.logAction("settings.class_mode", "ui.settings", "class mode ${if (on) "on" else "off"}")
    }
    ListGroup("In class", footer = "From the timetable the app has saved. It can switch a few minutes late.") {
        GroupItem(
            "Silence during classes",
            "settings.class_mode",
            Lucide.BellOff,
            "Do Not Disturb turns on when a class starts and off when it ends",
            onClick = { set(!Settings.classMode) },
            trailing = { ItemSwitch(Settings.classMode, { set(it) }) },
            on = Settings.classMode,
        )
        if (Settings.classMode && !granted) GroupItem(
            "Allow Do Not Disturb access",
            "settings.class_mode.allow",
            Lucide.ExternalLink,
            "Nothing is silenced until you allow vitapmate",
            onClick = allow,
            tone = AppTheme.tones.warning,
        )
    }
}
