package vitapmate

import android.appwidget.AppWidgetProviderInfo
import android.content.Context
import android.content.Intent
import android.os.Build
import androidx.annotation.DrawableRes
import androidx.collection.intSetOf
import androidx.compose.runtime.Composable
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.ColorFilter
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
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.appWidgetBackground
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.updateAll
import androidx.glance.background
import androidx.glance.layout.Alignment
import androidx.glance.layout.Box
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.RowScope
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
import kotlin.reflect.KClass
import vitapmate.core.Core
import vitapmate.data.AppCore
import vitapmate.data.Settings
import vitapmate.data.coreFlow
import vitapmate.observability.Log

/**
 * The home-screen and lock-screen widgets (Glance, in the wallpaper's colours on Android 12+):
 * [TimetableWidget], [AttendanceWidget], [ExamsWidget] and [MessWidget], each from what the phone
 * already has (no network) in its own flow, and the pieces they share. Refreshed every 30 min (each
 * one's res/xml/<name>_widget.xml), after a background sync run and when the app goes to the
 * background.
 */
object Widgets {
    /** Below this height (dp) a widget shows one item. */
    const val TALL_DP = 120

    /** Narrower than this (dp), a one-row widget drops its badge. */
    const val BADGE_DP = 170

    /** Rows past the hero that fit; the rest clip at the widget's edge anyway. */
    const val MAX_ROWS = 8

    /** Set once the pickers' previews are published; renamed when a widget's look changes, so they publish again. */
    private const val PREVIEW_SET = "preview_set_widgets"

    private val receivers: List<Pair<GlanceAppWidget, KClass<out GlanceAppWidgetReceiver>>> = listOf(
        TimetableWidget to TimetableWidgetReceiver::class,
        AttendanceWidget to AttendanceWidgetReceiver::class,
        ExamsWidget to ExamsWidgetReceiver::class,
        MessWidget to MessWidgetReceiver::class,
    )

    /** Redraws the placed widgets; on Android 15+ also publishes the pickers' previews (home and lock screen), once per install. */
    suspend fun refresh(context: Context) {
        receivers.forEach { it.first.updateAll(context) }
        val prefs = context.getSharedPreferences("widget", Context.MODE_PRIVATE)
        if (Build.VERSION.SDK_INT < 35 || prefs.getBoolean(PREVIEW_SET, false)) return
        val manager = GlanceAppWidgetManager(context)
        val categories = intSetOf(AppWidgetProviderInfo.WIDGET_CATEGORY_HOME_SCREEN, AppWidgetProviderInfo.WIDGET_CATEGORY_KEYGUARD)
        if (receivers.all { manager.setWidgetPreviews(it.second, categories) == GlanceAppWidgetManager.SET_WIDGET_PREVIEWS_RESULT_SUCCESS }) {
            prefs.edit().putBoolean(PREVIEW_SET, true).apply()
        }
    }

    /** [block] on the cached pages in flow `widget.<name>`; null (logged as a warning) when the core or the cache fails. */
    suspend fun <T> cached(context: Context, name: String, block: suspend (Core, String) -> T): T? {
        Settings.load(context.applicationContext)
        AppCore.start(context)
        return runCatching { coreFlow("widget.$name", block) }
            .onFailure { Log.warn("widget", "$name unavailable: ${it::class.simpleName}") }.getOrNull()
    }
}

/** The widget's root: full size, rounded, opening the app on feature [open] when tapped. */
@Composable
fun widgetRoot(open: String): GlanceModifier {
    val intent = Intent(LocalContext.current, MainActivity::class.java).putExtra(MainActivity.OPEN, open)
        .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
    return GlanceModifier.fillMaxSize().appWidgetBackground().cornerRadius(android.R.dimen.system_app_widget_background_radius)
        .clickable(actionStartActivity(intent))
}

/** Whether the widget is wide enough for its badge. */
@Composable
fun widgetBadge() = LocalSize.current.width >= Widgets.BADGE_DP.dp

/** Whether the widget is tall enough for its rows. */
@Composable
fun widgetTall() = LocalSize.current.height >= Widgets.TALL_DP.dp

/** Nothing to show: [icon] in a badge (wide enough) and [text]. */
@Composable
fun WidgetMessage(open: String, @DrawableRes icon: Int, text: String) {
    val c = GlanceTheme.colors
    val badge = widgetBadge()
    Row(widgetRoot(open).background(c.widgetBackground).padding(start = if (badge) 8.dp else 14.dp, end = 14.dp), verticalAlignment = Alignment.CenterVertically) {
        if (badge) Cookie(c.primaryContainer) {
            Image(ImageProvider(icon), null, GlanceModifier.size(22.dp), colorFilter = ColorFilter.tint(c.onPrimaryContainer))
        }
        Text(text, style = TextStyle(c.onSurface, 14.5.sp, FontWeight.Medium), maxLines = 2)
    }
}

/** The filled block's colour and its ink: primary, or error when [danger]. */
@Composable
fun blockColors(danger: Boolean) = GlanceTheme.colors.let { if (danger) it.error to it.onError else it.primary to it.onPrimary }

/** One row high: [badge] in a cookie (wide enough), then [label], [name] and [chips], on a filled block (error when [danger]). */
@Composable
fun WidgetShort(open: String, label: String, name: String, badge: @Composable () -> Unit, danger: Boolean = false, chips: @Composable () -> Unit = {}) {
    val (fill, ink) = blockColors(danger)
    val wide = widgetBadge()
    Row(widgetRoot(open).background(fill).padding(start = if (wide) 8.dp else 14.dp, end = 14.dp), verticalAlignment = Alignment.CenterVertically) {
        if (wide) Cookie(ink) { badge() }
        Column {
            Text(label, style = TextStyle(ink, 12.sp, FontWeight.Medium), maxLines = 1)
            Text(name, style = TextStyle(ink, 16.5.sp, FontWeight.Bold), maxLines = 1)
            chips()
        }
    }
}

/** A badge's two lines: [big] bold over [small] (when given), in the block's colour (error when [danger]). */
@Composable
fun BadgeText(big: String, small: String?, danger: Boolean = false) {
    val ink = blockColors(danger).first
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(big, style = TextStyle(ink, 14.sp, FontWeight.Bold), maxLines = 1)
        if (small != null) Text(small, style = TextStyle(ink, 9.5.sp, FontWeight.Medium), maxLines = 1)
    }
}

/** Two rows or more: the hero ([pill], [range], [name], [chips]) filled (error when [danger]), [heading] (when not empty), then [rows]. */
@Composable
fun WidgetTall(open: String, pill: String, range: String, name: String, chips: @Composable () -> Unit, heading: String, danger: Boolean = false, rows: @Composable () -> Unit) {
    val c = GlanceTheme.colors
    val (fill, ink) = blockColors(danger)
    Column(widgetRoot(open).background(c.widgetBackground).padding(8.dp)) {
        Column(GlanceModifier.fillMaxWidth().cornerRadius(22.dp).background(fill).padding(horizontal = 14.dp, vertical = 12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(pill, GlanceModifier.cornerRadius(12.dp).background(ink).padding(horizontal = 9.dp, vertical = 3.dp), TextStyle(fill, 11.5.sp, FontWeight.Bold), maxLines = 1)
                Text(range, GlanceModifier.padding(start = 8.dp), TextStyle(ink, 12.5.sp, FontWeight.Medium), maxLines = 1)
            }
            Text(name, GlanceModifier.padding(top = 3.dp), TextStyle(ink, 20.sp, FontWeight.Bold), maxLines = 1)
            chips()
        }
        if (heading.isNotEmpty()) Text(heading, GlanceModifier.padding(start = 10.dp, end = 10.dp, top = 9.dp, bottom = 5.dp), TextStyle(c.onSurfaceVariant, 12.5.sp, FontWeight.Medium))
        Column(GlanceModifier.fillMaxWidth()) { rows() }
    }
}

/**
 * Row [i] of [count] in a segmented list (rounder at the list's ends); [edge] (the hero's row)
 * draws it over an accent copy of itself, 3 dp in, leaving the accent edge.
 */
@Composable
fun WidgetRow(i: Int, count: Int, edge: Boolean, content: @Composable RowScope.() -> Unit) {
    val c = GlanceTheme.colors
    val shape = when {
        count == 1 -> R.drawable.widget_row_single
        i == 0 -> R.drawable.widget_row_first
        i == count - 1 -> R.drawable.widget_row_last
        else -> R.drawable.widget_row_mid
    }
    fun tinted(color: ColorProvider) = GlanceModifier.background(ImageProvider(shape), colorFilter = ColorFilter.tint(color))
    Box(GlanceModifier.fillMaxWidth().height(if (i == 0) 31.dp else 33.dp).padding(top = if (i == 0) 0.dp else 2.dp)) {
        Row((if (edge) tinted(c.primary).padding(start = 3.dp) else GlanceModifier).fillMaxSize()) {
            Row(tinted(c.surfaceVariant).fillMaxSize().padding(start = if (edge) 9.dp else 12.dp, end = 12.dp), verticalAlignment = Alignment.CenterVertically, content = content)
        }
    }
}

/** The 54 dp scalloped badge in [fill], with [content] on it. */
@Composable
fun Cookie(fill: ColorProvider, content: @Composable () -> Unit) {
    Box(
        GlanceModifier.size(54.dp).background(ImageProvider(R.drawable.widget_cookie), colorFilter = ColorFilter.tint(fill)),
        contentAlignment = Alignment.Center,
    ) { content() }
    Spacer(GlanceModifier.width(10.dp))
}

/** [place] with a pin (hidden when empty) and [rest] ("ends 2:50 PM", a seat), as chips at [size]. */
@Composable
fun Chips(place: String, rest: List<String>, size: TextUnit) {
    if (place.isEmpty() && rest.isEmpty()) return
    val c = GlanceTheme.colors
    val chip = GlanceModifier.cornerRadius(10.dp).background(c.primaryContainer).padding(horizontal = 7.dp, vertical = 2.dp)
    val style = TextStyle(c.onPrimaryContainer, size, FontWeight.Medium)
    Row(GlanceModifier.padding(top = 5.dp), verticalAlignment = Alignment.CenterVertically) {
        if (place.isNotEmpty()) {
            Row(chip, verticalAlignment = Alignment.CenterVertically) {
                Image(ImageProvider(R.drawable.widget_pin), null, GlanceModifier.size(12.dp), colorFilter = ColorFilter.tint(c.onPrimaryContainer))
                Text(place, GlanceModifier.padding(start = 4.dp), style, maxLines = 1)
            }
        }
        rest.forEachIndexed { i, text ->
            if (i > 0 || place.isNotEmpty()) Spacer(GlanceModifier.width(5.dp))
            Text(text, chip, style, maxLines = 1)
        }
    }
}
