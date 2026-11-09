package vitapmate

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.text.format.DateFormat
import android.view.View
import android.widget.RemoteViews
import kotlinx.coroutines.MainScope
import kotlinx.coroutines.launch
import vitapmate.core.ClassKind
import vitapmate.core.ClassSlot
import vitapmate.data.AppCore
import vitapmate.data.Settings
import vitapmate.data.coreFlow
import vitapmate.features.timetable.istDay
import vitapmate.features.timetable.istMinute
import vitapmate.features.timetable.meeting
import vitapmate.observability.Log
import vitapmate.ui.clock
import vitapmate.ui.dayLabel

/**
 * Home-screen widget from the cached timetable only (no network), drawn with RemoteViews: the
 * class on now or next, and at 2+ rows high that class's day (today's classes, or the next class
 * day's once today's are over). Refreshed every 30 min (res/xml/timetable_widget.xml), after a
 * background sync run and when the app goes to the background. Flow `widget.timetable`.
 */
object TimetableWidget {
    /** Below this height (dp) only the class on now or next shows. */
    private const val TALL_DP = 120

    /** The day's classes past the first that fit; the rest clip at the widget's edge anyway. */
    private const val MAX_ROWS = 8

    /** Set once the picker's preview is published; renamed when the widget's look changes, so it publishes again. */
    private const val PREVIEW_SET = "preview_set_remoteviews"

    /** The class on now or next and its day's classes (minutes since midnight, India time); [nextDay] names a later day. */
    private class Agenda(val now: Int, val day: List<ClassSlot>, val next: ClassSlot?, val nextDay: String?)

    /** Redraws the placed widgets; on Android 15+ also publishes the picker's preview, once per install. */
    suspend fun refresh(context: Context) {
        val manager = AppWidgetManager.getInstance(context)
        val ids = manager.getAppWidgetIds(ComponentName(context, TimetableWidgetReceiver::class.java))
        if (ids.isNotEmpty()) {
            val agenda = load(context)
            for (id in ids) manager.updateAppWidget(id, views(context, agenda, tall(manager.getAppWidgetOptions(id))))
        }
        val prefs = context.getSharedPreferences("widget", Context.MODE_PRIVATE)
        if (Build.VERSION.SDK_INT < 35 || prefs.getBoolean(PREVIEW_SET, false)) return
        if (manager.setWidgetPreview(ComponentName(context, TimetableWidgetReceiver::class.java), android.appwidget.AppWidgetProviderInfo.WIDGET_CATEGORY_HOME_SCREEN, preview(context))) {
            prefs.edit().putBoolean(PREVIEW_SET, true).apply()
        }
    }

    /** One widget after a resize: shows or hides the day to match its new height. */
    internal suspend fun resized(context: Context, id: Int, options: Bundle) {
        AppWidgetManager.getInstance(context).updateAppWidget(id, views(context, load(context), tall(options)))
    }

    private fun tall(options: Bundle): Boolean = options.getInt(AppWidgetManager.OPTION_APPWIDGET_MAX_HEIGHT, 0) >= TALL_DP

    private suspend fun load(context: Context): Agenda? {
        Settings.load(context.applicationContext)
        AppCore.start(context)
        val ms = System.currentTimeMillis()
        val today = istDay(ms)
        val now = istMinute(ms)
        return runCatching {
            coreFlow("widget.timetable") { c, f ->
                val days = c.cachedTimetable(f)?.let { c.timetableDays(f, today, 8) } ?: return@coreFlow null
                val meet = days.map { d -> d.day to meeting(d).map { it.slot } }
                val todays = meet.firstOrNull { it.first == today }?.second.orEmpty()
                val later = todays.firstOrNull { it.end > now }?.let { today to it }
                    ?: meet.firstOrNull { it.first > today && it.second.isNotEmpty() }?.let { it.first to it.second.first() }
                Log.info("widget", "timetable", f, mapOf("today" to "${todays.size}", "next_in_days" to "${later?.let { it.first - today }}"))
                val day = later?.first?.takeIf { it != today }
                Agenda(now, day?.let { d -> meet.first { it.first == d }.second } ?: todays, later?.second, day?.let { dayLabel(it, today) })
            }
        }.onFailure { Log.warn("widget", "timetable unavailable: ${it::class.simpleName}") }.getOrNull()
    }

    private fun preview(context: Context): RemoteViews {
        fun slot(start: Int, name: String, room: String) = ClassSlot(0, start, start + 50, "", name, ClassKind.THEORY, room, "", "", "", null)
        val today = listOf(slot(540, "Data Structures", "AB1 204"), slot(600, "Operating Systems", "CB 312"), slot(840, "Computer Networks", "AB2 107"))
        return views(context, Agenda(560, today, today[0], null), tall = true, h24 = false)
    }

    private fun views(context: Context, agenda: Agenda?, tall: Boolean, h24: Boolean = DateFormat.is24HourFormat(context)): RemoteViews {
        val v = RemoteViews(context.packageName, R.layout.widget_timetable)
        val open = Intent().setComponent(ComponentName(context, "vitapmate.MainActivity")).putExtra("open", "timetable")
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        v.setOnClickPendingIntent(R.id.widget_root, PendingIntent.getActivity(context, 0, open, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT))
        v.removeAllViews(R.id.widget_list)
        val next = agenda?.next
        if (next == null) {
            v.setTextViewText(R.id.widget_label, "")
            v.setTextViewText(R.id.widget_name, if (agenda == null) "Open Vitap Mate to load your timetable" else "No classes coming up")
            for (id in listOf(R.id.widget_label, R.id.widget_room, R.id.widget_day, R.id.widget_list)) v.setViewVisibility(id, View.GONE)
            return v
        }
        val label = agenda.nextDay ?: if (next.start <= agenda.now) "Now" else "Next"
        v.setViewVisibility(R.id.widget_label, View.VISIBLE)
        v.setTextViewText(R.id.widget_label, "$label · ${clock(next.start, h24)} – ${clock(next.end, h24)}")
        v.setTextViewText(R.id.widget_name, next.name)
        v.setTextViewText(R.id.widget_room, next.room)
        v.setViewVisibility(R.id.widget_room, if (next.room.isEmpty()) View.GONE else View.VISIBLE)
        val showDay = tall && agenda.day.isNotEmpty()
        v.setViewVisibility(R.id.widget_day, if (showDay) View.VISIBLE else View.GONE)
        v.setViewVisibility(R.id.widget_list, if (showDay) View.VISIBLE else View.GONE)
        if (!showDay) return v
        v.setTextViewText(R.id.widget_day, agenda.nextDay ?: "Today")
        val muted = context.getColor(R.color.widget_muted)
        for (s in agenda.day.take(MAX_ROWS)) {
            val past = agenda.nextDay == null && s.end <= agenda.now
            val row = RemoteViews(context.packageName, R.layout.widget_timetable_row)
            row.setTextViewText(R.id.row_time, clock(s.start, h24))
            row.setTextViewText(R.id.row_name, s.name)
            if (past) row.setTextColor(R.id.row_name, muted)
            row.setTextViewText(R.id.row_room, s.room)
            row.setViewVisibility(R.id.row_room, if (s.room.isEmpty()) View.GONE else View.VISIBLE)
            v.addView(R.id.widget_list, row)
        }
        return v
    }
}

/** Registered in the app's manifest with res/xml/timetable_widget.xml. */
class TimetableWidgetReceiver : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) = async(context) { TimetableWidget.refresh(it) }

    override fun onAppWidgetOptionsChanged(context: Context, manager: AppWidgetManager, id: Int, options: Bundle) =
        async(context) { TimetableWidget.resized(it, id, options) }

    private fun async(context: Context, work: suspend (Context) -> Unit) {
        val pending = goAsync()
        val app = context.applicationContext
        MainScope().launch {
            try {
                work(app)
            } finally {
                pending.finish()
            }
        }
    }
}
