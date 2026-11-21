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
import vitapmate.features.timetable.istDay
import vitapmate.features.timetable.istMinute
import vitapmate.observability.Log
import vitapmate.ui.clock
import vitapmate.ui.formatDay

/**
 * Home-screen widget from the cached exam schedule: a countdown to the next paper ("CAT-II in 4
 * days") with its course, date, time, venue and seat; 2+ rows high, the papers after it. Flow
 * `widget.exams`; see [Widgets].
 */
object ExamsWidget : GlanceAppWidget() {
    /** A paper to come: its exam ("CAT-II"), course, day (since epoch) and time ("9:30 AM", or VTOP's "FN"). */
    private class Paper(val exam: String, val name: String, val day: Int, val time: String, val venue: String, val seat: String)

    /** The papers to come, soonest first, on [today] (India time). */
    private class Schedule(val today: Int, val papers: List<Paper>)

    override val sizeMode = SizeMode.Exact

    override val previewSizeMode = SizeMode.Responsive(setOf(DpSize(250.dp, 180.dp)))

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val schedule = load(context, DateFormat.is24HourFormat(context))
        provideContent { GlanceTheme { Widget(schedule) } }
    }

    override suspend fun providePreview(context: Context, widgetCategory: Int) {
        val today = istDay(System.currentTimeMillis())
        val papers = listOf(
            Paper("CAT-II", "Operating Systems", today + 4, "9:30 AM", "AB1 204", "R6 · C6"),
            Paper("CAT-II", "Computer Networks", today + 5, "2:00 PM", "CB 312", ""),
            Paper("CAT-II", "Data Structures", today + 7, "9:30 AM", "AB2 107", ""),
        )
        provideContent { GlanceTheme { Widget(Schedule(today, papers)) } }
    }

    private suspend fun load(context: Context, h24: Boolean): Schedule? {
        val ms = System.currentTimeMillis()
        val today = istDay(ms)
        val now = istMinute(ms)
        return Widgets.cached(context, "exams") { c, f ->
            val groups = c.cachedExams(f)?.groups ?: return@cached null
            val papers = groups.flatMap { g -> g.papers.map { g.name to it } }
                .filter { (_, p) -> p.day.let { it != null && (it > today || it == today && (p.end ?: p.start ?: 1440) > now) } }
                .sortedWith(compareBy({ it.second.day }, { it.second.start }))
                .map { (exam, p) ->
                    val seat = p.seat.ifEmpty { p.seatNo.takeIf { it.isNotEmpty() }?.let { "Seat $it" }.orEmpty() }
                    Paper(exam, p.name, p.day!!, p.start?.let { clock(it, h24) } ?: p.session, p.venue, seat)
                }
            Log.info("widget", "exams", f, mapOf("coming" to "${papers.size}", "next_in_days" to "${papers.firstOrNull()?.let { it.day - today }}"))
            Schedule(today, papers)
        }
    }

    /** "today", "tomorrow" or "in 4 days". */
    private fun countdown(days: Int) = when (days) {
        0 -> "today"
        1 -> "tomorrow"
        else -> "in $days days"
    }

    @Composable
    private fun Widget(schedule: Schedule?) {
        val next = schedule?.papers?.firstOrNull()
        if (next == null) {
            WidgetMessage("exams", R.drawable.widget_calendar, if (schedule == null) "Open Vitap Mate to load your exams" else "No exams coming up")
            return
        }
        val inDays = countdown(next.day - schedule.today)
        if (!widgetTall()) {
            val label = listOf("${next.exam} $inDays", next.time).filter { it.isNotEmpty() }.joinToString(" · ")
            WidgetShort("exams", label, next.name, { BadgeText(formatDay(next.day, "d"), formatDay(next.day, "MMM")) }) {
                Chips(next.venue, listOf(next.seat).filter { it.isNotEmpty() }, 11.5.sp)
            }
            return
        }
        val c = GlanceTheme.colors
        val range = "$inDays · ${formatDay(next.day, "EEE d MMM")}"
        WidgetTall("exams", next.exam, range, next.name, { Chips(next.venue, listOf(next.time, next.seat).filter { it.isNotEmpty() }, 12.sp) }, if (schedule.papers.size > 1) "Coming up" else "") {
            val rows = schedule.papers.drop(1).take(Widgets.MAX_ROWS)
            rows.forEachIndexed { i, p ->
                WidgetRow(i, rows.size, edge = false) {
                    Text(formatDay(p.day, "EEE d"), GlanceModifier.width(66.dp), TextStyle(c.onSurfaceVariant, 12.sp, FontWeight.Medium), maxLines = 1)
                    Text(p.name, GlanceModifier.defaultWeight(), TextStyle(c.onSurface, 13.5.sp, FontWeight.Medium), maxLines = 1)
                    if (p.time.isNotEmpty()) Text(p.time, GlanceModifier.padding(start = 10.dp), TextStyle(c.onSurfaceVariant, 12.sp), maxLines = 1)
                }
            }
        }
    }
}

/** Registered in the app's manifest with res/xml/exams_widget.xml. */
class ExamsWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = ExamsWidget
}
