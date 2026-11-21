package vitapmate

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.appwidget.AppWidgetProviderInfo
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.graphics.Typeface
import android.os.Build
import android.os.Bundle
import android.text.SpannableString
import android.text.Spanned
import android.text.format.DateFormat
import android.text.style.StyleSpan
import android.text.style.TypefaceSpan
import android.util.SizeF
import android.util.TypedValue
import android.view.View
import android.widget.RemoteViews
import androidx.annotation.ColorRes
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
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
 * Home-screen widget from the cached timetable only (no network), drawn with RemoteViews
 * (res/layout/widget_*.xml) in the wallpaper's colours (res/values/widget.xml, with night and v31
 * copies): the class on now or next, and at 2+ rows high that class's day (today's classes, or the
 * next class day's once today's are over). Refreshed every 30 min (res/xml/timetable_widget.xml), after a background
 * sync run and when the app goes to the background. Flow `widget.timetable`.
 */
object TimetableWidget {
    /** Below this height (dp) only the class on now or next shows. */
    private const val TALL_DP = 120

    /** Narrower than this (dp), the one-row widget drops its badge. */
    private const val BADGE_DP = 170

    /** The day's classes past the first that fit; the rest clip at the widget's edge anyway. */
    private const val MAX_ROWS = 8

    /** Set once the picker's preview is published; renamed when the widget's look changes, so it publishes again. */
    private const val PREVIEW_SET = "preview_set_views"

    /** The class on now or next and its day's classes (minutes since midnight, India time); [nextDay] names a later day. */
    private class Agenda(val now: Int, val day: List<ClassSlot>, val next: ClassSlot?, val nextDay: String?)

    /** Redraws the placed widgets; on Android 15+ also publishes the picker's preview, once per install. */
    suspend fun refresh(context: Context) {
        val manager = AppWidgetManager.getInstance(context)
        val provider = ComponentName(context, TimetableWidgetReceiver::class.java)
        val ids = manager.getAppWidgetIds(provider)
        val h24 = DateFormat.is24HourFormat(context)
        if (ids.isNotEmpty()) {
            val agenda = load(context)
            ids.forEach { manager.updateAppWidget(it, sized(manager.getAppWidgetOptions(it)) { w, h -> draw(context, agenda, h24, w, h) }) }
        }
        val prefs = context.getSharedPreferences("widget", Context.MODE_PRIVATE)
        if (Build.VERSION.SDK_INT < 35 || prefs.getBoolean(PREVIEW_SET, false)) return
        fun slot(start: Int, name: String, room: String) = ClassSlot(0, start, start + 50, "", name, ClassKind.THEORY, room, "", "", "", null)
        val today = listOf(slot(540, "Data Structures", "AB1 204"), slot(600, "Operating Systems", "CB 312"), slot(840, "Computer Networks", "AB2 107"))
        // The picker draws the preview at the widget's 3×2 drop size.
        val preview = draw(context, Agenda(560, today, today[0], null), h24 = false, 250f, 180f)
        if (manager.setWidgetPreview(provider, AppWidgetProviderInfo.WIDGET_CATEGORY_HOME_SCREEN, preview)) prefs.edit().putBoolean(PREVIEW_SET, true).apply()
    }

    /** One drawing per size the launcher gives (Android 12+), else landscape and portrait from the min and max sizes. */
    private fun sized(options: Bundle, draw: (Float, Float) -> RemoteViews): RemoteViews {
        if (Build.VERSION.SDK_INT >= 31) {
            @Suppress("DEPRECATION") // The typed overload is Android 13+.
            val sizes = options.getParcelableArrayList<SizeF>(AppWidgetManager.OPTION_APPWIDGET_SIZES)
            if (!sizes.isNullOrEmpty()) return RemoteViews(sizes.associateWith { draw(it.width, it.height) })
        }
        fun dp(key: String) = options.getInt(key).toFloat()
        val landscape = draw(dp(AppWidgetManager.OPTION_APPWIDGET_MAX_WIDTH), dp(AppWidgetManager.OPTION_APPWIDGET_MIN_HEIGHT))
        val portrait = draw(dp(AppWidgetManager.OPTION_APPWIDGET_MIN_WIDTH), dp(AppWidgetManager.OPTION_APPWIDGET_MAX_HEIGHT))
        return RemoteViews(landscape, portrait)
    }

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

    /** The widget at [width] × [height] dp. */
    private fun draw(context: Context, agenda: Agenda?, h24: Boolean, width: Float, height: Float): RemoteViews {
        val px = context.resources.displayMetrics.density
        fun Int.dp() = (this * px).toInt()
        val badge = width >= BADGE_DP
        val next = agenda?.next
        val layout = when {
            next == null -> R.layout.widget_message
            height < TALL_DP -> R.layout.widget_short
            else -> R.layout.widget_tall
        }
        val v = RemoteViews(context.packageName, layout)
        val open = Intent(context, MainActivity::class.java).putExtra("open", "timetable")
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        v.setOnClickPendingIntent(android.R.id.background, PendingIntent.getActivity(context, 0, open, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT))
        if (layout != R.layout.widget_tall) {
            v.setViewVisibility(R.id.widget_badge, if (badge) View.VISIBLE else View.GONE)
            v.setViewPadding(android.R.id.background, (if (badge) 8 else 14).dp(), 0, 14.dp(), 0)
        }
        if (next == null) {
            v.setTextViewText(R.id.widget_text, if (agenda == null) "Open Vitap Mate to load your timetable" else "No classes coming up")
            return v
        }
        val now = agenda.nextDay == null && next.start <= agenda.now
        val label = agenda.nextDay ?: if (now) "Now" else "Next"
        val range = "${clock(next.start, h24)} – ${clock(next.end, h24)}"
        val ends = if (now) "ends ${clock(next.end, h24)}" else null
        v.setTextViewText(R.id.widget_name, next.name)
        chips(v, next.room, ends, if (layout == R.layout.widget_short) 11.5f else 12f)
        if (layout == R.layout.widget_short) {
            val (time, half) = clock(next.start, h24).split(" ").let { it[0] to it.getOrNull(1) }
            v.setTextViewText(R.id.widget_time, time)
            v.setViewVisibility(R.id.widget_half, if (half != null) View.VISIBLE else View.GONE)
            v.setTextViewText(R.id.widget_half, half.orEmpty())
            v.setTextViewText(R.id.widget_label, "$label · $range")
            return v
        }
        v.setTextViewText(R.id.widget_label, label)
        v.setTextViewText(R.id.widget_range, range)
        v.setTextViewText(R.id.widget_day, agenda.nextDay ?: "Today")
        val rows = agenda.day.take(MAX_ROWS)
        v.removeAllViews(R.id.widget_rows)
        rows.forEachIndexed { i, s ->
            val row = RemoteViews(context.packageName, R.layout.widget_row)
            val shape = when {
                rows.size == 1 -> R.drawable.widget_row_single
                i == 0 -> R.drawable.widget_row_first
                i == rows.lastIndex -> R.drawable.widget_row_last
                else -> R.drawable.widget_row_mid
            }
            val cur = s == next
            val past = agenda.nextDay == null && s.end <= agenda.now
            if (i == 0) row.setViewPadding(R.id.widget_row, 0, 0, 0, 0)
            row.setImageViewResource(R.id.widget_row_fill, shape)
            if (cur) {
                row.setImageViewResource(R.id.widget_row_edge, shape)
                row.setViewVisibility(R.id.widget_row_edge, View.VISIBLE)
                row.setViewPadding(R.id.widget_row_body, 3.dp(), 0, 0, 0)
                row.setViewPadding(R.id.widget_row_text, 9.dp(), 0, 12.dp(), 0)
            }
            // Classes already over drawn fainter, the one on now or next bold.
            val muted = if (past) R.color.widget_outline else R.color.widget_on_surface_variant
            row.setTextViewText(R.id.widget_row_time, weighted(clock(s.start, h24), if (cur) BOLD else MEDIUM))
            row.color(context, R.id.widget_row_time, if (cur) R.color.widget_primary else muted)
            row.setTextViewText(R.id.widget_row_name, weighted(s.name, if (cur) BOLD else if (past) NORMAL else MEDIUM))
            row.color(context, R.id.widget_row_name, if (past) R.color.widget_outline else R.color.widget_on_surface)
            row.setViewVisibility(R.id.widget_row_room, if (s.room.isNotEmpty()) View.VISIBLE else View.GONE)
            row.setTextViewText(R.id.widget_row_room, weighted(s.room, if (cur) BOLD else NORMAL))
            row.color(context, R.id.widget_row_room, muted)
            v.addView(R.id.widget_rows, row)
        }
        return v
    }

    /** The room with a pin (hidden when empty) and [ends] ("ends 2:50 PM", while a class is on), as chips at [size] sp. */
    private fun chips(v: RemoteViews, room: String, ends: String?, size: Float) {
        v.setViewVisibility(R.id.widget_chips, if (room.isEmpty() && ends == null) View.GONE else View.VISIBLE)
        v.setViewVisibility(R.id.widget_room_chip, if (room.isNotEmpty()) View.VISIBLE else View.GONE)
        v.setViewVisibility(R.id.widget_chip_gap, if (room.isNotEmpty() && ends != null) View.VISIBLE else View.GONE)
        v.setViewVisibility(R.id.widget_ends, if (ends != null) View.VISIBLE else View.GONE)
        v.setTextViewText(R.id.widget_room, room)
        v.setTextViewText(R.id.widget_ends, ends.orEmpty())
        v.setTextViewTextSize(R.id.widget_room, TypedValue.COMPLEX_UNIT_SP, size)
        v.setTextViewTextSize(R.id.widget_ends, TypedValue.COMPLEX_UNIT_SP, size)
    }

    private const val NORMAL = 0
    private const val MEDIUM = 1
    private const val BOLD = 2

    /** [text] in a font weight that changes with the row's state (a layout's fixed weights are in its XML). */
    private fun weighted(text: String, weight: Int): CharSequence = SpannableString(text).apply {
        when (weight) {
            BOLD -> setSpan(StyleSpan(Typeface.BOLD), 0, length, Spanned.SPAN_INCLUSIVE_EXCLUSIVE)
            MEDIUM -> setSpan(TypefaceSpan("sans-serif-medium"), 0, length, Spanned.SPAN_INCLUSIVE_EXCLUSIVE)
        }
    }

    /** [id]'s text colour from [color]; resolved by the launcher on Android 12+, so it follows light and dark mode. */
    private fun RemoteViews.color(context: Context, id: Int, @ColorRes color: Int) {
        if (Build.VERSION.SDK_INT >= 31) setColor(id, "setTextColor", color) else setTextColor(id, context.getColor(color))
    }
}

/** Registered in the app's manifest with res/xml/timetable_widget.xml; draws off the main thread. */
class TimetableWidgetReceiver : AppWidgetProvider() {
    override fun onUpdate(context: Context, manager: AppWidgetManager, ids: IntArray) = redraw(context)

    override fun onAppWidgetOptionsChanged(context: Context, manager: AppWidgetManager, id: Int, options: Bundle) = redraw(context)

    private fun redraw(context: Context) {
        val pending = goAsync()
        CoroutineScope(Dispatchers.Default).launch {
            try {
                TimetableWidget.refresh(context.applicationContext)
            } finally {
                pending.finish()
            }
        }
    }
}
