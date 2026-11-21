package vitapmate

import android.content.Context
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
import kotlin.math.roundToInt
import vitapmate.core.ClassKind
import vitapmate.observability.Log

/**
 * Home-screen widget from the cached attendance: the courses most at risk first, each with its
 * percentage and advice, in the error colour below 75%; one row high, the lowest only. Flow
 * `widget.attendance`; see [Widgets].
 */
object AttendanceWidget : GlanceAppWidget() {
    /** One course as drawn: "72%", "Data Structures", "Lab", "Attend 2 labs". */
    private class Line(val percent: String, val name: String, val kind: String, val code: String, val advice: String, val safe: Boolean)

    override val sizeMode = SizeMode.Exact

    override val previewSizeMode = SizeMode.Responsive(setOf(DpSize(250.dp, 180.dp)))

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val lines = load(context)
        provideContent { GlanceTheme { Widget(lines) } }
    }

    override suspend fun providePreview(context: Context, widgetCategory: Int) {
        val lines = listOf(
            Line("71%", "Operating Systems", "Theory", "CSE3003", "Attend 3", false),
            Line("78%", "Computer Networks", "Lab", "CSE3001", "Don't skip", true),
            Line("84%", "Data Structures", "Theory", "CSE2001", "Can skip 4", true),
        )
        provideContent { GlanceTheme { Widget(lines) } }
    }

    private suspend fun load(context: Context): List<Line>? = Widgets.cached(context, "attendance") { c, f ->
        val courses = c.cachedAttendance(f)?.courses ?: return@cached null
        // Debarred, then below 75% by the classes to attend, then by the classes to spare; courses with none held last.
        val risky = courses.sortedWith(compareBy({ it.total == 0 }, { !it.debarred }, { it.safe }, { it.canSkip - it.mustAttend }, { it.percent }))
        Log.info("widget", "attendance", f, mapOf("courses" to "${courses.size}", "unsafe" to "${courses.count { !it.safe }}"))
        risky.map {
            val percent = if (it.total == 0) "–" else "${it.percent.roundToInt()}%"
            Line(percent, it.name, if (it.kind == ClassKind.LAB) "Lab" else "Theory", it.code, if (it.debarred) "Debarred" else it.advice, it.safe)
        }
    }

    @Composable
    private fun Widget(lines: List<Line>?) {
        val top = lines?.firstOrNull()
        if (top == null) {
            WidgetMessage("attendance", R.drawable.widget_user_check, if (lines == null) "Open Vitap Mate to load your attendance" else "No attendance yet")
            return
        }
        if (!widgetTall()) {
            WidgetShort("attendance", top.advice, top.name, { BadgeText(top.percent, top.kind, !top.safe) }, !top.safe)
            return
        }
        val c = GlanceTheme.colors
        WidgetTall("attendance", top.percent, top.advice, top.name, { Chips("", listOf(top.code, top.kind), 12.sp) }, if (lines.size > 1) "Attendance" else "", !top.safe) {
            val rows = lines.drop(1).take(Widgets.MAX_ROWS)
            rows.forEachIndexed { i, l ->
                val tone = if (l.safe) c.onSurfaceVariant else c.error
                WidgetRow(i, rows.size, edge = false) {
                    Text(l.percent, GlanceModifier.width(44.dp), TextStyle(tone, 12.sp, if (l.safe) FontWeight.Medium else FontWeight.Bold), maxLines = 1)
                    Text(
                        if (l.kind == "Lab") "${l.name} · Lab" else l.name,
                        GlanceModifier.defaultWeight(),
                        TextStyle(c.onSurface, 13.5.sp, FontWeight.Medium),
                        maxLines = 1,
                    )
                    Text(l.advice, GlanceModifier.padding(start = 10.dp), TextStyle(tone, 12.sp, if (l.safe) FontWeight.Normal else FontWeight.Bold), maxLines = 1)
                }
            }
        }
    }
}

/** Registered in the app's manifest with res/xml/attendance_widget.xml. */
class AttendanceWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = AttendanceWidget
}
