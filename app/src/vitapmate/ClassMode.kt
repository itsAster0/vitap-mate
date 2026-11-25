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
import vitapmate.data.istDay
import vitapmate.data.istMinute
import vitapmate.data.istSecond
import vitapmate.features.timetable.meeting
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.rememberHaptics

/**
 * Settings → In class (Android 10+, off by default): a Do Not Disturb rule of the app's own
 * ("In class", priority only) that is on while a class from the cached timetable is on, or an
 * exam from the cached schedule (from its reporting time to its end). An inexact alarm
 * ([LiveUpdateReceiver]) flips it at each start and end, so it can be a few minutes late; the
 * student can change or remove the rule in Android's Modes. Flows `classmode.*`.
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
            // The classes and exams of the next 8 days, by day and start.
            val quiet = runCatching {
                withContext(Dispatchers.IO) {
                    val c = AppCore.await()
                    val classes = c.cachedTimetable(f)?.let { c.timetableDays(f, today, 8).flatMap { d -> meeting(d).map { Quiet(d.day, it.slot.start, it.slot.end, it.slot.name) } } }.orEmpty()
                    val exams = c.cachedExams(f)?.groups.orEmpty().flatMap { g ->
                        g.papers.mapNotNull { p ->
                            val day = p.day?.takeIf { it in today until today + 8 } ?: return@mapNotNull null
                            val start = p.start ?: return@mapNotNull null
                            Quiet(day, p.reporting?.coerceAtMost(start) ?: start, p.end ?: (start + 180), "${g.name}, ${p.name}")
                        }
                    }
                    (classes + exams).sortedWith(compareBy({ it.day }, { it.start }))
                }
            }.onFailure { Log.warn("classmode", "timetable unavailable: ${it::class.simpleName}", f) }.getOrNull().orEmpty()
            val todays = quiet.filter { it.day == today }
            val current = todays.filter { it.start <= now && now < it.end }.maxByOrNull { it.end }
            // Minutes from today's midnight to the next flip.
            val wake = current?.end ?: quiet.firstOrNull { it.day > today || it.start > now }?.let { (it.day - today) * 1440 + it.start }
            // A rule made before Android 15's typed rules is updated once, in place.
            val rule = id?.also { if (Build.VERSION.SDK_INT >= 35 && manager.getAutomaticZenRule(it).type != AutomaticZenRule.TYPE_SCHEDULE_TIME) manager.updateAutomaticZenRule(it, inClass(app)) }
                ?: manager.addAutomaticZenRule(inClass(app))
            manager.setAutomaticZenRuleState(rule, Condition(CONDITION, current?.name ?: "No class now", if (current != null) Condition.STATE_TRUE else Condition.STATE_FALSE))
            if (wake != null) alarms.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, ms + (wake * 60L - istSecond(ms)) * 1000, alarm)
            Log.info("classmode", "rule", f, mapOf("on" to "${current != null}", "today" to "${todays.size}", "wake_in_min" to "${wake?.minus(now)}"))
        }
    }

    /** The "In class" rule: on Android 15+ a time-schedule mode with its icon and what turns it on. */
    private fun inClass(app: Context): AutomaticZenRule {
        val settings = ComponentName(app, MainActivity::class.java)
        if (Build.VERSION.SDK_INT < 35) return AutomaticZenRule("In class", null, settings, CONDITION, null, NotificationManager.INTERRUPTION_FILTER_PRIORITY, true)
        return AutomaticZenRule.Builder("In class", CONDITION)
            .setConfigurationActivity(settings)
            .setInterruptionFilter(NotificationManager.INTERRUPTION_FILTER_PRIORITY)
            .setType(AutomaticZenRule.TYPE_SCHEDULE_TIME)
            .setIconResId(R.drawable.tile_class)
            .setTriggerDescription("During your classes, and exams from reporting time")
            .build()
    }
}

/** A class or exam on [day] (days since 1970), from [start] to [end] (minutes since midnight). */
private data class Quiet(val day: Int, val start: Int, val end: Int, val name: String)

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
    ListGroup("In class", footer = "From the timetable and exam schedule the app has saved. It can switch a few minutes late.") {
        GroupItem(
            "Silence during classes and exams",
            "settings.class_mode",
            Lucide.BellOff,
            "Do Not Disturb is on during each class, and for an exam from reporting time until it ends",
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
