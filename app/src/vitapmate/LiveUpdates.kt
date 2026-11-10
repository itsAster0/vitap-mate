package vitapmate

import android.Manifest
import android.app.AlarmManager
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.text.SpannableString
import android.text.Spanned
import android.text.format.DateFormat
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.core.ClassSlot
import vitapmate.core.OutingRequest
import vitapmate.core.OutingStanding
import vitapmate.data.AppCore
import vitapmate.data.Settings
import vitapmate.features.timetable.istDay
import vitapmate.features.timetable.istMinute
import vitapmate.features.timetable.istSecond
import vitapmate.features.timetable.meeting
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.clock

/**
 * Settings → Live updates (Android 16+, off by default): promoted ongoing notifications in
 * `ProgressStyle` on their own channel. The class on now (or next) from the cached timetable, with
 * its room and a bar until it ends, then the next one, gone after the day's last class; and an
 * outing request while VTOP has it pending, then approved or rejected once a fetched outing page
 * says so. The class one is redrawn by an inexact alarm ([LiveUpdateReceiver]) at each class
 * boundary and every [TICK_MIN] minutes during the day. Flows `live.*`.
 */
object LiveUpdates {
    private const val CHANNEL = "live"
    private const val CLASS_ID = 300

    /** The next class shows from this many minutes before the day's first one. */
    private const val LEAD_MIN = 30

    /** How often the bar moves on (the countdown runs on its own). */
    private const val TICK_MIN = 5

    fun supported(): Boolean = Build.VERSION.SDK_INT >= 36

    /** Whether the system shows them as Live updates (the student can turn that off per app). */
    fun promotable(context: Context): Boolean =
        Build.VERSION.SDK_INT >= 36 && context.getSystemService(NotificationManager::class.java).canPostPromotedNotifications()

    private fun allowed(context: Context) = supported() && Settings.liveUpdates &&
        context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) == PackageManager.PERMISSION_GRANTED

    /** Brings the class update and its alarm in step with the setting and the cached timetable. */
    suspend fun refresh(context: Context, flowName: String) = flow(flowName) { f ->
        val app = context.applicationContext
        val manager = app.getSystemService(NotificationManager::class.java)
        if (!allowed(app)) {
            app.getSystemService(AlarmManager::class.java).cancel(alarm(app))
            if (supported()) manager.activeNotifications.forEach { if (it.notification.channelId == CHANNEL) manager.cancel(it.id) }
            return@flow
        }
        channel(manager)
        val ms = System.currentTimeMillis()
        val today = istDay(ms)
        val now = istMinute(ms)
        // Days from today with their classes, start order; null without a cached timetable.
        val days = runCatching {
            withContext(Dispatchers.IO) {
                val c = AppCore.await()
                c.cachedTimetable(f)?.let { c.timetableDays(f, today, 8).map { d -> d.day to meeting(d).map { it.slot }.sortedBy { it.start } } }
            }
        }.onFailure { Log.warn("live", "timetable unavailable: ${it::class.simpleName}", f) }.getOrNull()
        val todays = days?.firstOrNull { it.first == today }?.second.orEmpty()
        val current = todays.firstOrNull { it.start <= now && now < it.end }
        val next = todays.firstOrNull { it.start > now }
        val before = todays.lastOrNull { it.end <= now }
        val showing: String
        // Minutes from today's midnight to the next redraw.
        val wake: Int?
        if (current != null) {
            showing = "now"
            post(app, manager, current, true, current.start, current.end, todays.firstOrNull { it.start >= current.end }, ms, now)
            wake = minOf(current.end, now + TICK_MIN)
        } else if (next != null && (before != null || next.start - now <= LEAD_MIN)) {
            showing = "next"
            post(app, manager, next, false, before?.end ?: (next.start - LEAD_MIN), next.start, null, ms, now)
            wake = minOf(next.start, now + TICK_MIN)
        } else {
            showing = "none"
            manager.cancel(CLASS_ID)
            wake = next?.let { it.start - LEAD_MIN }
                ?: days?.firstOrNull { it.first > today && it.second.isNotEmpty() }?.let { (it.first - today) * 1440 + it.second.first().start - LEAD_MIN }
        }
        if (wake != null) app.getSystemService(AlarmManager::class.java).setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, atMinute(ms, wake), alarm(app))
        Log.info("live", "class", f, mapOf("today" to "${todays.size}", "showing" to showing, "wake_in_min" to "${wake?.minus(now)}"))
    }

    /**
     * Compares the cached outing requests with the last look: one pending for a trip still ahead
     * (VTOP keeps old ones pending) shows as pending, one that went from pending to approved or
     * rejected shows its outcome for an hour, one gone (cancelled) goes.
     */
    suspend fun outings(context: Context, flowName: String) = flow(flowName) { f ->
        val app = context.applicationContext
        if (!allowed(app)) return@flow
        val requests = runCatching {
            withContext(Dispatchers.IO) {
                val c = AppCore.await()
                c.cachedGeneralOuting(f)?.requests.orEmpty() + c.cachedWeekendOuting(f)?.requests.orEmpty()
            }
        }.onFailure { Log.warn("live", "outings unavailable: ${it::class.simpleName}", f) }.getOrNull() ?: return@flow
        val prefs = app.getSharedPreferences("live", Context.MODE_PRIVATE)
        val seen = prefs.getStringSet("outings", null)?.associate { it.substringBefore('=') to it.substringAfter('=') }
        val manager = app.getSystemService(NotificationManager::class.java)
        channel(manager)
        val now = requests.associateBy { "${it.kind.name}.${it.key}" }
        var posted = 0
        for ((key, r) in now) {
            val was = seen?.get(key)
            if (r.standing == OutingStanding.PENDING && r.highlight == null) {
                manager.cancel(outingId(key))
            } else if (r.standing == OutingStanding.PENDING || (was == OutingStanding.PENDING.name && r.standing.name != was)) {
                // Pending for a trip still ahead (also newly applied), or just decided.
                postOuting(app, manager, key, r)
                posted++
            }
        }
        seen?.forEach { (key, was) -> if (was == OutingStanding.PENDING.name && key !in now) manager.cancel(outingId(key)) }
        prefs.edit().putStringSet("outings", now.map { (k, r) -> "$k=${r.standing.name}" }.toSet()).apply()
        Log.info("live", "outings", f, mapOf("requests" to "${requests.size}", "posted" to "$posted", "first" to "${seen == null}"))
    }

    /** Developer → Preview: a class on now (half way) and an approved outing, from made-up data. */
    fun preview(context: Context) {
        val app = context.applicationContext
        if (!allowed(app)) return
        val manager = app.getSystemService(NotificationManager::class.java)
        channel(manager)
        val ms = System.currentTimeMillis()
        val now = istMinute(ms)
        fun slot(start: Int, name: String, room: String) = ClassSlot(0, start, start + 50, "", name, vitapmate.core.ClassKind.THEORY, room, "", "", "", null)
        post(app, manager, slot(now - 25, "Data Structures", "AB1 204"), true, now - 25, now + 25, slot(now + 35, "Operating Systems", "CB 312"), ms, now)
        postOuting(app, manager, "preview", OutingRequest("preview", vitapmate.core.OutingKind.GENERAL, "Vijayawada", "Shopping", "Approved", OutingStanding.APPROVED, "APPROVED", listOf(vitapmate.core.OutingWhen("Out", "Sat 14 Nov, 9:00 AM")), "IN 4 DAYS", null, null))
    }

    private fun channel(manager: NotificationManager) {
        manager.createNotificationChannel(
            NotificationChannel(CHANNEL, "Live updates", NotificationManager.IMPORTANCE_DEFAULT).apply {
                description = "The class on now and outing requests"
                setSound(null, null)
                enableVibration(false)
            },
        )
    }

    /** The class [slot], on now ([live]) or next: the bar runs from [from] to [to] (minutes), [then] after it. */
    private fun post(context: Context, manager: NotificationManager, slot: ClassSlot, live: Boolean, from: Int, to: Int, then: ClassSlot?, ms: Long, now: Int) {
        if (Build.VERSION.SDK_INT < 36) return
        val h24 = DateFormat.is24HourFormat(context)
        val text = listOfNotNull(
            slot.room.ifEmpty { null },
            "${clock(slot.start, h24)} – ${clock(slot.end, h24)}",
            then?.let { "then ${it.name} at ${clock(it.start, h24)}" },
        ).joinToString(" · ")
        val style = Notification.ProgressStyle()
            .setProgressSegments(listOf(Notification.ProgressStyle.Segment((to - from).coerceAtLeast(1))))
            .setProgress((now - from).coerceIn(0, (to - from).coerceAtLeast(1)))
            .setStyledByProgress(true)
        val n = Notification.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(if (live) slot.name else "Next: ${slot.name}")
            .setContentText(text)
            .setSubText(if (live) "Class on now" else "Next class")
            .setStyle(style)
            .setShortCriticalText(slot.room.ifEmpty { clock(slot.start, h24) })
            .setWhen(atMinute(ms, to))
            .setShowWhen(true)
            .setUsesChronometer(true)
            .setChronometerCountDown(true)
            .setOngoing(true)
            .setOnlyAlertOnce(true)
            .setRequestPromotedOngoing(true)
            .setCategory(Notification.CATEGORY_EVENT)
            // Gone on its own if the next alarm is late.
            .setTimeoutAfter(atMinute(ms, to) - ms + 10 * 60_000L)
            .setContentIntent(open(context, "timetable", CLASS_ID))
            .build()
        manager.notify(CLASS_ID, n)
    }

    /** An outing request: pending (half the bar), approved or rejected (all of it, in Android 17's safe or danger colour). */
    private fun postOuting(context: Context, manager: NotificationManager, key: String, r: OutingRequest) {
        if (Build.VERSION.SDK_INT < 36) return
        val pending = r.standing == OutingStanding.PENDING
        val word = when (r.standing) {
            OutingStanding.PENDING -> "pending"
            OutingStanding.APPROVED -> "approved"
            OutingStanding.REJECTED -> "rejected"
            OutingStanding.OTHER -> r.status.lowercase()
        }
        val semantic = if (Build.VERSION.SDK_INT >= 37) when (r.standing) {
            OutingStanding.APPROVED -> Notification.SEMANTIC_STYLE_SAFE
            OutingStanding.REJECTED -> Notification.SEMANTIC_STYLE_DANGER
            OutingStanding.PENDING -> Notification.SEMANTIC_STYLE_CAUTION
            OutingStanding.OTHER -> Notification.SEMANTIC_STYLE_UNSPECIFIED
        } else null
        val kind = if (r.kind == vitapmate.core.OutingKind.WEEKEND) "Weekend outing" else "Outing"
        val title = SpannableString("$kind $word")
        if (semantic != null && Build.VERSION.SDK_INT >= 37) title.setSpan(Notification.createSemanticStyleAnnotation(semantic), kind.length + 1, title.length, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
        val decided = Notification.ProgressStyle.Segment(1)
        if (semantic != null && !pending && Build.VERSION.SDK_INT >= 37) decided.setSemanticStyle(semantic)
        val style = Notification.ProgressStyle()
            .setProgressSegments(listOf(Notification.ProgressStyle.Segment(1), decided))
            .setProgress(if (pending) 1 else 2)
            .setStyledByProgress(true)
        val b = Notification.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(title)
            .setContentText(listOfNotNull(r.place.ifEmpty { null }, r.`when`.firstOrNull()?.text).joinToString(" · "))
            .setSubText(if (pending) r.status else "VTOP: ${r.status}")
            .setStyle(style)
            .setShortCriticalText(word.replaceFirstChar { it.uppercase() })
            .setOngoing(true)
            .setOnlyAlertOnce(pending)
            .setRequestPromotedOngoing(true)
            .setCategory(Notification.CATEGORY_STATUS)
            .setContentIntent(open(context, "outing", outingId(key)))
        if (!pending) b.setAutoCancel(true).setTimeoutAfter(60 * 60_000L)
        manager.notify(outingId(key), b.build())
    }

    private fun outingId(key: String) = 400 + (key.hashCode() and 0xffff)

    private fun open(context: Context, feature: String, request: Int) = PendingIntent.getActivity(
        context, request,
        Intent(context, MainActivity::class.java).putExtra(MainActivity.OPEN, feature).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

    private fun alarm(context: Context) = PendingIntent.getBroadcast(
        context, 0, Intent(context, LiveUpdateReceiver::class.java), PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
    )

    /** Wall-clock ms of [minute] minutes after today's midnight in India (the timetable's clock). */
    private fun atMinute(ms: Long, minute: Int): Long = ms + (minute * 60L - istSecond(ms)) * 1000
}

/** The class update's alarm and [ClassMode]'s, and after a reboot or an update (alarms don't survive them). */
class LiveUpdateReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val pending = goAsync()
        val app = context.applicationContext
        MainScope().launch {
            try {
                Settings.load(app)
                AppCore.start(app)
                LiveUpdates.refresh(app, "live.alarm")
                ClassMode.refresh(app, "classmode.alarm")
            } finally {
                pending.finish()
            }
        }
    }
}
