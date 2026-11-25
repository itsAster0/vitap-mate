package vitapmate

import android.content.Context
import android.text.format.DateFormat
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.SizeMode
import androidx.glance.appwidget.provideContent
import androidx.glance.layout.padding
import androidx.glance.layout.width
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import vitapmate.core.ClassKind
import vitapmate.core.ClassSlot
import vitapmate.data.istDay
import vitapmate.data.istMinute
import vitapmate.features.timetable.meeting
import vitapmate.observability.Log
import vitapmate.ui.clock
import vitapmate.ui.dayLabel

/**
 * Home-screen widget from the cached timetable: the class on now or next, and at 2+ rows high
 * that class's day (today's classes, or the next class day's once today's are over). Flow
 * `widget.timetable`; see [Widgets].
 */
object TimetableWidget : GlanceAppWidget() {
    /** The class on now or next and its day's classes (minutes since midnight, India time); [nextDay] names a later day. */
    private class Agenda(val now: Int, val day: List<ClassSlot>, val next: ClassSlot?, val nextDay: String?)

    override val sizeMode = SizeMode.Exact

    /** The picker draws the preview at the widget's 3×2 drop size. */
    override val previewSizeMode = SizeMode.Responsive(setOf(DpSize(250.dp, 180.dp)))

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val agenda = load(context)
        val h24 = DateFormat.is24HourFormat(context)
        provideContent { GlanceTheme { Widget(agenda, h24) } }
    }

    override suspend fun providePreview(context: Context, widgetCategory: Int) {
        fun slot(start: Int, name: String, room: String) = ClassSlot(0, start, start + 50, "", name, ClassKind.THEORY, room, "", "", "", null)
        val today = listOf(slot(540, "Data Structures", "AB1 204"), slot(600, "Operating Systems", "CB 312"), slot(840, "Computer Networks", "AB2 107"))
        provideContent { GlanceTheme { Widget(Agenda(560, today, today[0], null), h24 = false) } }
    }

    private suspend fun load(context: Context): Agenda? {
        val ms = System.currentTimeMillis()
        val today = istDay(ms)
        val now = istMinute(ms)
        return Widgets.cached(context, "timetable") { c, f ->
            val days = c.cachedTimetable(f)?.let { c.timetableDays(f, today, 8) } ?: return@cached null
            val meet = days.map { d -> d.day to meeting(d).map { it.slot } }
            val todays = meet.firstOrNull { it.first == today }?.second.orEmpty()
            val later = todays.firstOrNull { it.end > now }?.let { today to it }
                ?: meet.firstOrNull { it.first > today && it.second.isNotEmpty() }?.let { it.first to it.second.first() }
            Log.info("widget", "timetable", f, mapOf("today" to "${todays.size}", "next_in_days" to "${later?.let { it.first - today }}"))
            val day = later?.first?.takeIf { it != today }
            Agenda(now, day?.let { d -> meet.first { it.first == d }.second } ?: todays, later?.second, day?.let { dayLabel(it, today) })
        }
    }

    @Composable
    private fun Widget(agenda: Agenda?, h24: Boolean) {
        val next = agenda?.next
        if (next == null) {
            WidgetMessage("timetable", R.drawable.widget_calendar, if (agenda == null) "Open Vitap Mate to load your timetable" else "No classes coming up")
            return
        }
        val now = agenda.nextDay == null && next.start <= agenda.now
        val label = agenda.nextDay ?: if (now) "Now" else "Next"
        val range = "${clock(next.start, h24)} – ${clock(next.end, h24)}"
        val ends = listOfNotNull(if (now) "ends ${clock(next.end, h24)}" else null)
        if (!widgetTall()) {
            val (time, half) = clock(next.start, h24).split(" ").let { it[0] to it.getOrNull(1) }
            WidgetShort("timetable", "$label · $range", next.name, { BadgeText(time, half) }) { Chips(next.room, ends, 11.5.sp) }
            return
        }
        val c = GlanceTheme.colors
        WidgetTall("timetable", label, range, next.name, { Chips(next.room, ends, 12.sp) }, agenda.nextDay ?: "Today") {
            val rows = agenda.day.take(Widgets.MAX_ROWS)
            rows.forEachIndexed { i, s ->
                val cur = s == next
                val past = agenda.nextDay == null && s.end <= agenda.now
                WidgetRow(i, rows.size, cur) {
                    // Classes already over drawn fainter, the one on now or next bold.
                    val muted = if (past) c.outline else c.onSurfaceVariant
                    Text(clock(s.start, h24), GlanceModifier.width(66.dp), TextStyle(if (cur) c.primary else muted, 12.sp, if (cur) FontWeight.Bold else FontWeight.Medium), maxLines = 1)
                    Text(
                        s.name,
                        GlanceModifier.defaultWeight(),
                        TextStyle(if (past) c.outline else c.onSurface, 13.5.sp, if (cur) FontWeight.Bold else if (past) FontWeight.Normal else FontWeight.Medium),
                        maxLines = 1,
                    )
                    if (s.room.isNotEmpty()) Text(s.room, GlanceModifier.padding(start = 10.dp), TextStyle(muted, 12.sp, if (cur) FontWeight.Bold else FontWeight.Normal), maxLines = 1)
                }
            }
        }
    }
}

/** Registered in the app's manifest with res/xml/timetable_widget.xml; keeps its name so placed widgets carry on. */
class TimetableWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = TimetableWidget
}
