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
import vitapmate.data.Docs
import vitapmate.data.Meal
import vitapmate.data.MealWindow
import vitapmate.data.MessTimings
import vitapmate.data.currentOrNextMeal
import vitapmate.data.epochDayOf
import vitapmate.data.messMenuYear
import vitapmate.data.minuteOf
import vitapmate.data.parseMessMenu
import vitapmate.data.readSpreadsheet
import vitapmate.observability.Log
import vitapmate.ui.clock
import vitapmate.ui.formatDay

/**
 * Home-screen widget from the mess menu set in Docs (the newest spreadsheet shown as the menu, on
 * the sheet picked there) and Settings → Mess Timings: the meal on now or next, and at 2+ rows high
 * that day's meals (tomorrow's after dinner). Flow `widget.mess`; see [Widgets].
 */
object MessWidget : GlanceAppWidget() {
    /** One meal as drawn: its serving hours and what's on ("Idli, Sambar, Chutney"). */
    private class Line(val meal: Meal, val window: MealWindow, val items: String)

    /** [day]'s meals, [next] on now or next, at [now] (minutes since local midnight); [tomorrow] once today's are over. */
    private class Menu(val now: Int, val meals: List<Line>, val next: Meal, val tomorrow: Boolean)

    /** The Docs menu's Regular/Special choice (MessMenuView's key). */
    private const val SHEET_KEY = "mess_menu_sheet"

    override val sizeMode = SizeMode.Exact

    override val previewSizeMode = SizeMode.Responsive(setOf(DpSize(250.dp, 180.dp)))

    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val menu = load(context)
        val h24 = DateFormat.is24HourFormat(context)
        provideContent { GlanceTheme { Widget(menu, h24) } }
    }

    override suspend fun providePreview(context: Context, widgetCategory: Int) {
        val items = mapOf(Meal.Breakfast to "Idli, Sambar, Coconut Chutney", Meal.Lunch to "Jeera Rice, Dal Tadka, Paneer Butter Masala", Meal.Snacks to "Samosa, Tea", Meal.Dinner to "Chapati, Veg Kurma, Curd Rice")
        val lines = Meal.entries.map { Line(it, MessTimings.windows.getValue(it), items.getValue(it)) }
        provideContent { GlanceTheme { Widget(Result.success(Menu(770, lines, Meal.Lunch, false)), h24 = false) } }
    }

    /** Null without a menu; a failure when the menu has no entry for the day. */
    private suspend fun load(context: Context): Result<Menu>? = Widgets.cached(context, "mess") { _, f ->
        val doc = Docs.docs.filter { it.asMessMenu }.maxByOrNull { it.addedAt } ?: return@cached null
        val ms = System.currentTimeMillis()
        val minute = minuteOf(ms)
        val windows = MessTimings.windows
        val next = currentOrNextMeal(minute, windows)
        val day = epochDayOf(ms) + if (next == null) 1 else 0
        val menu = parseMessMenu(Docs.withFile(doc, ::readSpreadsheet)) { month ->
            messMenuYear(month, doc.name, formatDay(day, "yyyy").toInt(), formatDay(day, "M").toInt())
        }
        val chosen = context.getSharedPreferences("settings", Context.MODE_PRIVATE).getString(SHEET_KEY, null)
        val sheet = menu.sheets.firstOrNull { it.name == chosen } ?: menu.sheets.first()
        val dom = formatDay(day, "d").toInt()
        val inMenu = (menu.month == null || menu.month == formatDay(day, "M").toInt()) && sheet.days.containsKey(dom)
        Log.info("widget", "mess", f, mapOf("sheets" to "${menu.sheets.size}", "in_menu" to "$inMenu", "meal" to "${next ?: Meal.Breakfast}"))
        if (!inMenu) return@cached Result.failure(NoSuchElementException(formatDay(day, "d MMMM")))
        val lines = Meal.entries.map { m -> Line(m, windows.getValue(m), sheet.itemsFor(dom, m).joinToString(", ") { it.name }) }
        Result.success(Menu(minute, lines, next ?: Meal.Breakfast, next == null))
    }

    @Composable
    private fun Widget(result: Result<Menu>?, h24: Boolean) {
        val menu = result?.getOrNull()
        if (menu == null) {
            val text = if (result == null) "Open Vitap Mate to set up the mess menu" else "No mess menu for ${result.exceptionOrNull()?.message}"
            WidgetMessage("docs", R.drawable.widget_utensils, text)
            return
        }
        val next = menu.meals.first { it.meal == menu.next }
        val now = !menu.tomorrow && menu.now >= next.window.start
        val label = if (menu.tomorrow) "Tomorrow" else if (now) "Now" else "Next"
        val range = "${clock(next.window.start, h24)} – ${clock(next.window.end, h24)}"
        val items = next.items.ifEmpty { "Not on the menu" }
        if (!widgetTall()) {
            val (time, half) = clock(next.window.start, h24).split(" ").let { it[0] to it.getOrNull(1) }
            WidgetShort("docs", "${next.meal.label} · $label", items, { BadgeText(time, half) }) {
                Chips("", listOfNotNull(if (now) "ends ${clock(next.window.end, h24)}" else null), 11.5.sp)
            }
            return
        }
        val c = GlanceTheme.colors
        val whatsOn: @Composable () -> Unit = { Text(items, GlanceModifier.padding(top = 3.dp), TextStyle(c.onPrimary, 13.sp), maxLines = 2) }
        WidgetTall("docs", label, range, next.meal.label, whatsOn, if (menu.tomorrow) "Tomorrow" else "Today") {
            menu.meals.forEachIndexed { i, l ->
                val cur = l.meal == menu.next
                val past = !menu.tomorrow && l.meal.ordinal < menu.next.ordinal
                WidgetRow(i, menu.meals.size, cur) {
                    // Meals already over drawn fainter, the one on now or next bold.
                    val muted = if (past) c.outline else c.onSurfaceVariant
                    Text(clock(l.window.start, h24), GlanceModifier.width(66.dp), TextStyle(if (cur) c.primary else muted, 12.sp, if (cur) FontWeight.Bold else FontWeight.Medium), maxLines = 1)
                    Text(l.meal.label, GlanceModifier.width(72.dp), TextStyle(if (past) c.outline else c.onSurface, 13.5.sp, if (cur) FontWeight.Bold else FontWeight.Medium), maxLines = 1)
                    Text(l.items, GlanceModifier.defaultWeight(), TextStyle(muted, 12.sp), maxLines = 1)
                }
            }
        }
    }
}

/** Registered in the app's manifest with res/xml/mess_widget.xml. */
class MessWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = MessWidget
}
