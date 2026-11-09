package vitapmate.features.timetable

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Build
import android.text.format.DateFormat
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.LocalSize
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetManager
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.SizeMode
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.lazy.LazyColumn
import androidx.glance.appwidget.lazy.items
import androidx.glance.appwidget.provideContent
import androidx.glance.appwidget.updateAll
import androidx.glance.background
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.width
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import vitapmate.core.ClassKind
import vitapmate.core.ClassSlot
import vitapmate.data.AppCore
import vitapmate.data.Settings
import vitapmate.data.coreFlow
import vitapmate.observability.Log
import vitapmate.ui.clock
import vitapmate.ui.dayLabel

/**
 * Home-screen widget from the cached timetable only (no network): the class on now or next, and
 * at 2+ rows high that class's day (today's classes, or the next class day's once today's are over). Refreshed every 30 min (res/xml/timetable_widget.xml), after a
 * background sync run and when the app goes to the background. Flow `widget.timetable`.
 */
object TimetableWidget : GlanceAppWidget() {
    private val SMALL = DpSize(110.dp, 48.dp)
    private val TALL = DpSize(180.dp, 140.dp)

    override val sizeMode = SizeMode.Responsive(setOf(SMALL, TALL))
    override val previewSizeMode = SizeMode.Responsive(setOf(SMALL, TALL))

    /** The class on now or next and its day's classes (minutes since midnight, India time); [nextDay] names a later day. */
    private class Agenda(val now: Int, val day: List<ClassSlot>, val next: ClassSlot?, val nextDay: String?)

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        Settings.load(context.applicationContext)
        AppCore.start(context)
        val ms = System.currentTimeMillis()
        val today = istDay(ms)
        val now = istMinute(ms)
        val agenda = runCatching {
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
        val h24 = DateFormat.is24HourFormat(context)
        provideContent { GlanceTheme { Content(context, agenda, h24) } }
    }

    /** Redraws the placed widgets; on Android 15+ also publishes the picker's preview, once per install. */
    suspend fun refresh(context: Context) {
        updateAll(context)
        val prefs = context.getSharedPreferences("widget", Context.MODE_PRIVATE)
        if (Build.VERSION.SDK_INT < 35 || prefs.getBoolean("preview_set", false)) return
        if (GlanceAppWidgetManager(context).setWidgetPreviews(TimetableWidgetReceiver::class) == GlanceAppWidgetManager.SET_WIDGET_PREVIEWS_RESULT_SUCCESS) prefs.edit().putBoolean("preview_set", true).apply()
    }

    override suspend fun providePreview(context: Context, widgetCategory: Int) {
        fun slot(start: Int, name: String, room: String) = ClassSlot(0, start, start + 50, "", name, ClassKind.THEORY, room, "", "", "", null)
        val today = listOf(slot(540, "Data Structures", "AB1 204"), slot(600, "Operating Systems", "CB 312"), slot(840, "Computer Networks", "AB2 107"))
        provideContent { GlanceTheme { Content(context, Agenda(560, today, today[0], null), false) } }
    }

    @Composable
    private fun Content(context: Context, agenda: Agenda?, h24: Boolean) {
        val open = Intent().setComponent(ComponentName(context, "vitapmate.MainActivity")).putExtra("open", "timetable")
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        val c = GlanceTheme.colors
        val muted = TextStyle(color = c.onSurfaceVariant, fontSize = 12.sp)
        Column(GlanceModifier.fillMaxSize().background(c.widgetBackground).cornerRadius(16.dp).padding(12.dp).clickable(actionStartActivity(open))) {
            val next = agenda?.next
            if (next == null) {
                Text(if (agenda == null) "Open Vitap Mate to load your timetable" else "No classes coming up", style = TextStyle(color = c.onSurface, fontSize = 14.sp))
                return@Column
            }
            val label = agenda.nextDay ?: if (next.start <= agenda.now) "Now" else "Next"
            Text("$label · ${clock(next.start, h24)} – ${clock(next.end, h24)}", style = TextStyle(color = c.primary, fontSize = 12.sp, fontWeight = FontWeight.Medium), maxLines = 1)
            Text(next.name, style = TextStyle(color = c.onSurface, fontSize = 16.sp, fontWeight = FontWeight.Bold), maxLines = 1)
            if (next.room.isNotEmpty()) Text(next.room, style = muted, maxLines = 1)
            if (LocalSize.current.height < TALL.height || agenda.day.isEmpty()) return@Column
            Spacer(GlanceModifier.height(10.dp))
            Text(agenda.nextDay ?: "Today", style = TextStyle(color = c.onSurfaceVariant, fontSize = 12.sp, fontWeight = FontWeight.Medium))
            LazyColumn(GlanceModifier.fillMaxWidth()) {
                items(agenda.day, itemId = { it.start.toLong() }) { s ->
                    val past = agenda.nextDay == null && s.end <= agenda.now
                    Row(GlanceModifier.fillMaxWidth().padding(vertical = 3.dp)) {
                        Text(clock(s.start, h24), style = muted, maxLines = 1)
                        Spacer(GlanceModifier.width(8.dp))
                        Text(s.name, style = TextStyle(color = if (past) c.onSurfaceVariant else c.onSurface, fontSize = 13.sp), maxLines = 1, modifier = GlanceModifier.defaultWeight())
                        if (s.room.isNotEmpty()) {
                            Spacer(GlanceModifier.width(8.dp))
                            Text(s.room, style = muted, maxLines = 1)
                        }
                    }
                }
            }
        }
    }
}

/** Registered in the app's manifest with res/xml/timetable_widget.xml. */
class TimetableWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = TimetableWidget
}
