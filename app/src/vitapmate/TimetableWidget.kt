package vitapmate

import android.content.Context
import android.content.Intent
import android.os.Build
import android.text.format.DateFormat
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.DpSize
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.ColorFilter
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.GlanceTheme
import androidx.glance.Image
import androidx.glance.ImageProvider
import androidx.glance.LocalContext
import androidx.glance.LocalSize
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetManager
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.SizeMode
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.appWidgetBackground
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.provideContent
import androidx.glance.appwidget.updateAll
import androidx.glance.background
import androidx.glance.layout.Alignment
import androidx.glance.layout.Box
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.height
import androidx.glance.layout.padding
import androidx.glance.layout.size
import androidx.glance.layout.width
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import androidx.glance.unit.ColorProvider
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
 * Home-screen widget from the cached timetable only (no network), drawn with Glance in the
 * wallpaper's colours: the class on now or next, and at 2+ rows high that class's day (today's
 * classes, or the next class day's once today's are over). Refreshed every 30 min
 * (res/xml/timetable_widget.xml), after a background sync run and when the app goes to the
 * background. Flow `widget.timetable`.
 */
object TimetableWidget : GlanceAppWidget() {
    /** Below this height (dp) only the class on now or next shows. */
    private const val TALL_DP = 120

    /** Narrower than this (dp), the one-row widget drops its badge. */
    private const val BADGE_DP = 170

    /** The day's classes past the first that fit; the rest clip at the widget's edge anyway. */
    private const val MAX_ROWS = 8

    /** Set once the picker's preview is published; renamed when the widget's look changes, so it publishes again. */
    private const val PREVIEW_SET = "preview_set_glance"

    /** The class on now or next and its day's classes (minutes since midnight, India time); [nextDay] names a later day. */
    private class Agenda(val now: Int, val day: List<ClassSlot>, val next: ClassSlot?, val nextDay: String?)

    override val sizeMode = SizeMode.Exact

    /** The picker draws the preview at the widget's 3×2 drop size. */
    override val previewSizeMode = SizeMode.Responsive(setOf(DpSize(250.dp, 180.dp)))

    /** Redraws the placed widgets; on Android 15+ also publishes the picker's preview, once per install. */
    suspend fun refresh(context: Context) {
        updateAll(context)
        val prefs = context.getSharedPreferences("widget", Context.MODE_PRIVATE)
        if (Build.VERSION.SDK_INT < 35 || prefs.getBoolean(PREVIEW_SET, false)) return
        if (GlanceAppWidgetManager(context).setWidgetPreviews(TimetableWidgetReceiver::class) == GlanceAppWidgetManager.SET_WIDGET_PREVIEWS_RESULT_SUCCESS) {
            prefs.edit().putBoolean(PREVIEW_SET, true).apply()
        }
    }

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val agenda = load(context)
        val h24 = DateFormat.is24HourFormat(context)
        provideContent { Widget(agenda, h24) }
    }

    override suspend fun providePreview(context: Context, widgetCategory: Int) {
        fun slot(start: Int, name: String, room: String) = ClassSlot(0, start, start + 50, "", name, ClassKind.THEORY, room, "", "", "", null)
        val today = listOf(slot(540, "Data Structures", "AB1 204"), slot(600, "Operating Systems", "CB 312"), slot(840, "Computer Networks", "AB2 107"))
        provideContent { Widget(Agenda(560, today, today[0], null), h24 = false) }
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

    @Composable
    private fun Widget(agenda: Agenda?, h24: Boolean) = GlanceTheme {
        val c = GlanceTheme.colors
        val size = LocalSize.current
        val open = Intent(LocalContext.current, MainActivity::class.java).putExtra("open", "timetable")
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        val root = GlanceModifier.fillMaxSize().appWidgetBackground().cornerRadius(android.R.dimen.system_app_widget_background_radius)
            .clickable(actionStartActivity(open))
        val badge = size.width >= BADGE_DP.dp
        val next = agenda?.next
        if (next == null) {
            Row(root.background(c.widgetBackground).padding(start = if (badge) 8.dp else 14.dp, end = 14.dp), verticalAlignment = Alignment.CenterVertically) {
                if (badge) Cookie(c.primaryContainer) {
                    Image(ImageProvider(R.drawable.widget_calendar), null, GlanceModifier.size(22.dp), colorFilter = ColorFilter.tint(c.onPrimaryContainer))
                }
                val text = if (agenda == null) "Open Vitap Mate to load your timetable" else "No classes coming up"
                Text(text, style = TextStyle(c.onSurface, 14.5.sp, FontWeight.Medium), maxLines = 2)
            }
            return@GlanceTheme
        }
        val now = agenda.nextDay == null && next.start <= agenda.now
        val label = agenda.nextDay ?: if (now) "Now" else "Next"
        val range = "${clock(next.start, h24)} – ${clock(next.end, h24)}"
        // The class on now or next in a filled block; its label, room and end inverted on it.
        val block = c.primary
        val fg = c.onPrimary
        val accent = c.onPrimary
        val onAccent = c.primary
        val ends = if (now) "ends ${clock(next.end, h24)}" else null
        if (size.height < TALL_DP.dp) {
            Row(root.background(block).padding(start = if (badge) 8.dp else 14.dp, end = 14.dp), verticalAlignment = Alignment.CenterVertically) {
                if (badge) Cookie(accent) {
                    val (time, half) = clock(next.start, h24).split(" ").let { it[0] to it.getOrNull(1) }
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Text(time, style = TextStyle(onAccent, 14.sp, FontWeight.Bold), maxLines = 1)
                        if (half != null) Text(half, style = TextStyle(onAccent, 9.5.sp, FontWeight.Medium), maxLines = 1)
                    }
                }
                Column {
                    Text("$label · $range", style = TextStyle(fg, 12.sp, FontWeight.Medium), maxLines = 1)
                    Text(next.name, style = TextStyle(fg, 16.5.sp, FontWeight.Bold), maxLines = 1)
                    Chips(next.room, ends, 11.5.sp, c.primaryContainer, c.onPrimaryContainer)
                }
            }
            return@GlanceTheme
        }
        Column(root.background(c.widgetBackground).padding(8.dp)) {
            Column(GlanceModifier.fillMaxWidth().cornerRadius(22.dp).background(block).padding(horizontal = 14.dp, vertical = 12.dp)) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        label,
                        GlanceModifier.cornerRadius(12.dp).background(accent).padding(horizontal = 9.dp, vertical = 3.dp),
                        TextStyle(onAccent, 11.5.sp, FontWeight.Bold),
                        maxLines = 1,
                    )
                    Text(range, GlanceModifier.padding(start = 8.dp), TextStyle(fg, 12.5.sp, FontWeight.Medium), maxLines = 1)
                }
                Text(next.name, GlanceModifier.padding(top = 3.dp), TextStyle(fg, 20.sp, FontWeight.Bold), maxLines = 1)
                Chips(next.room, ends, 12.sp, c.primaryContainer, c.onPrimaryContainer)
            }
            Text(agenda.nextDay ?: "Today", GlanceModifier.padding(start = 10.dp, end = 10.dp, top = 9.dp, bottom = 5.dp), TextStyle(c.onSurfaceVariant, 12.5.sp, FontWeight.Medium))
            val rows = agenda.day.take(MAX_ROWS)
            Column(GlanceModifier.fillMaxWidth()) {
                rows.forEachIndexed { i, s ->
                    val shape = when {
                        rows.size == 1 -> R.drawable.widget_row_single
                        i == 0 -> R.drawable.widget_row_first
                        i == rows.lastIndex -> R.drawable.widget_row_last
                        else -> R.drawable.widget_row_mid
                    }
                    fun tinted(color: ColorProvider) = GlanceModifier.background(ImageProvider(shape), colorFilter = ColorFilter.tint(color))
                    val cur = s == next
                    val past = agenda.nextDay == null && s.end <= agenda.now
                    // The hero's class: the row over an accent copy of itself, 3 dp in, leaves the accent edge.
                    Box(GlanceModifier.fillMaxWidth().height(if (i == 0) 31.dp else 33.dp).padding(top = if (i == 0) 0.dp else 2.dp)) {
                        Row(
                            (if (cur) tinted(c.primary).padding(start = 3.dp) else GlanceModifier).fillMaxSize(),
                        ) {
                            Row(tinted(c.surfaceVariant).fillMaxSize().padding(start = if (cur) 9.dp else 12.dp, end = 12.dp), verticalAlignment = Alignment.CenterVertically) {
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
        }
    }

    /** The 54 dp scalloped badge in [fill], with [content] on it. */
    @Composable
    private fun Cookie(fill: ColorProvider, content: @Composable () -> Unit) {
        Box(
            GlanceModifier.size(54.dp).background(ImageProvider(R.drawable.widget_cookie), colorFilter = ColorFilter.tint(fill)),
            contentAlignment = Alignment.Center,
        ) { content() }
        Spacer(GlanceModifier.width(10.dp))
    }

    /** The room with a pin (hidden when empty) and [ends] ("ends 2:50 PM", while a class is on), as chips in [fill]. */
    @Composable
    private fun Chips(room: String, ends: String?, size: TextUnit, fill: ColorProvider, color: ColorProvider) {
        if (room.isEmpty() && ends == null) return
        val chip = GlanceModifier.cornerRadius(10.dp).background(fill).padding(horizontal = 7.dp, vertical = 2.dp)
        Row(GlanceModifier.padding(top = 5.dp), verticalAlignment = Alignment.CenterVertically) {
            if (room.isNotEmpty()) {
                Row(chip, verticalAlignment = Alignment.CenterVertically) {
                    Image(ImageProvider(R.drawable.widget_pin), null, GlanceModifier.size(12.dp), colorFilter = ColorFilter.tint(color))
                    Text(room, GlanceModifier.padding(start = 4.dp), TextStyle(color, size, FontWeight.Medium), maxLines = 1)
                }
            }
            if (ends != null) {
                if (room.isNotEmpty()) Spacer(GlanceModifier.width(5.dp))
                Text(ends, chip, TextStyle(color, size, FontWeight.Medium), maxLines = 1)
            }
        }
    }
}

/** Registered in the app's manifest with res/xml/timetable_widget.xml. */
class TimetableWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = TimetableWidget
}
