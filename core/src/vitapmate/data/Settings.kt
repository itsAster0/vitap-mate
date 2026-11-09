package vitapmate.data

import android.content.Context
import android.content.SharedPreferences
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.core.content.edit
import kotlin.reflect.KProperty

/** App settings (SharedPreferences "settings", the Flutter app's keys) as Compose-observable state. */
object Settings {
    private var prefs: SharedPreferences? = null
    private val all = mutableListOf<Pref<*>>()
    private val tuned = mutableStateMapOf<Tunable, Int>()
    private val notifyOff = mutableStateMapOf<String, Boolean>()

    /** One saved value: Compose state, read by [load] and written through on every set. */
    private class Pref<T>(val key: String, val default: T, val read: SharedPreferences.(String, T) -> T, val write: SharedPreferences.Editor.(String, T) -> Unit) {
        val state = mutableStateOf(default)
        operator fun getValue(owner: Any?, property: KProperty<*>): T = state.value
        operator fun setValue(owner: Any?, property: KProperty<*>, value: T) {
            state.value = value
            prefs?.edit { write(key, value) }
        }
        fun load(p: SharedPreferences) { state.value = p.read(key, default) }
    }

    private fun <T> pref(key: String, default: T, read: SharedPreferences.(String, T) -> T, write: SharedPreferences.Editor.(String, T) -> Unit) =
        Pref(key, default, read, write).also { all += it }

    private fun flag(key: String, default: Boolean) = pref(key, default, { k, d -> getBoolean(k, d) }, { k, v -> putBoolean(k, v) })

    private fun text(key: String) = pref<String?>(key, null, { k, d -> getString(k, d) }, { k, v -> putString(k, v) })

    /** An enum saved as its lowercase name. */
    private fun <T : Enum<*>?> choice(key: String, default: T, entries: List<T & Any>) =
        pref(key, default, { k, d -> entries.firstOrNull { it.name.lowercase() == getString(k, null) } ?: d }, { k, v -> putString(k, v?.name?.lowercase()) })

    /** Whether attendance shows how many classes can be skipped. */
    var showSkipAdvice by flag("settings_show_skip_advice", false)

    /**
     * Which one fills in VTOP's OTP: one at a time. Set when one is set up or connected, or chosen
     * with "Use this for OTPs"; null before either is. The other stays set up, on standby.
     */
    var otpSource by choice<OtpSource?>("settings_otp_source", null, OtpSource.entries)

    /** Haptic feedback on taps, toggles, refreshes and results (the system's touch feedback setting still applies). */
    var haptics by flag("settings_haptics", true)

    /**
     * The Flutter app's "Auto Refresh": pages fetch on their own (on open, when old, on return to
     * the app, the open-app sweep). Off, only a page with nothing saved fetches by itself.
     */
    var autoRefresh by flag("settings_auto_refresh", true)

    /** Failures of work the student didn't ask for (auto refresh, the sweep) show as toasts too; off, they only log. */
    var showBackgroundErrors by flag("settings_show_background_errors", false)

    /** Light, dark, or as the phone is set. */
    var themeMode by choice("settings_theme_mode", ThemeMode.System, ThemeMode.entries)

    /** The colours: [WALLPAPER], a palette's name (the design module's `Palette`), or null for the default. */
    var colorChoice by pref<String?>(
        "settings_colors",
        null,
        // The old wallpaper switch carries over.
        { k, d -> getString(k, d) ?: WALLPAPER.takeIf { getBoolean("settings_dynamic_color", false) } },
        { k, v -> putString(k, v) },
    )

    const val WALLPAPER = "wallpaper"

    /** Black backgrounds in dark mode (OLED screens). */
    var pureBlack by flag("settings_pure_black", false)

    /** The student's chosen avatar photo (a file in the app's private files), or null for the initial. */
    var avatarPath by text("settings_avatar_path")

    private var tabs by pref("settings_nav_tabs", NavTabs.DEFAULT, { k, _ -> NavTabs.decode(getString(k, null)) }, { k, v -> putString(k, NavTabs.encode(v)) })

    /**
     * The nav bar's tabs, feature ids in order (rules in [NavTabs]: at most 4, always "more").
     * Ids of features that no longer exist may be in it; the app skips them.
     */
    val navTabs: List<String> get() = tabs

    fun setNavTabs(ids: List<String>) { tabs = NavTabs.normalize(ids) }

    /** Pins [id]; false when the bar is full (4) and nothing changed. */
    fun addNavTab(id: String): Boolean = (id in navTabs || navTabs.size < NavTabs.MAX).also { tabs = NavTabs.add(navTabs, id) }

    fun removeNavTab(id: String) { tabs = NavTabs.remove(navTabs, id) }

    fun moveNavTab(id: String, to: Int) { tabs = NavTabs.move(navTabs, id, to) }

    /**
     * Developer listed in Settings, as on Android: turned on by tapping the version 7 times,
     * off from the switch at the top of Developer. Kept across launches.
     */
    var developerEnabled by flag("dev_always_show", false)

    /** Developer → Inspect Traffic: from the next launch, VTOP requests use the Wi-Fi proxy and trust user CAs. */
    var inspectTraffic by flag("dev_inspect_traffic", false)

    /**
     * Developer → HttpEngine, on by default: from the next launch, on Android 14+ (where the
     * phone has HttpEngine) and with Inspect Traffic off, requests go through [VtopHttpEngine]
     * (HTTP/3 where the server offers it); otherwise plain OkHttp.
     */
    var httpEngine by flag("dev_http_engine", true)

    /**
     * Background sync (WorkManager, every [syncHours]): off by default, and offered only while OTP
     * auto-fill works, since nobody can type a code in the background.
     */
    var backgroundSync by flag("settings_background_sync", false)

    /** Live updates (Android 16+): the class on now and outing requests as promoted notifications; off by default. */
    var liveUpdates by flag("settings_live_updates", false)

    /** Hours between background syncs, one of [SYNC_HOURS]. */
    var syncHours by pref("settings_sync_hours", SYNC_HOURS_DEFAULT, { k, d -> getInt(k, d).takeIf { it in SYNC_HOURS } ?: d }, { k, v -> putInt(k, v) })

    /**
     * Why background sync is paused ("Gmail needs reconnecting"), set when a background run
     * couldn't finish the OTP and cleared by the next good run; null when it isn't.
     */
    var syncPaused by text("settings_sync_paused")

    /** Whether background sync notifies about changes of [kind] ("attendance", "marks", "timetable", "exams", "grades"). */
    fun notifies(kind: String): Boolean = notifyOff[kind] != true

    fun setNotifies(kind: String, on: Boolean) {
        if (on) notifyOff.remove(kind) else notifyOff[kind] = true
        prefs?.edit { putBoolean("settings_notify_$kind", on) }
    }

    /** [t]'s value, in its unit. */
    fun tunable(t: Tunable): Int = tuned[t] ?: t.default

    /** Saves [value] (kept in [Tunable.range]) for [t]; null goes back to its default. */
    fun setTunable(t: Tunable, value: Int?) {
        val v = value?.coerceIn(t.range)
        if (v == null || v == t.default) tuned.remove(t) else tuned[t] = v
        prefs?.edit { if (v == null || v == t.default) remove(t.key) else putInt(t.key, v) }
    }

    /** Reads the saved values; call once at start. */
    fun load(context: Context) {
        val p = context.getSharedPreferences("settings", Context.MODE_PRIVATE)
        prefs = p
        all.forEach { it.load(p) }
        Tunable.entries.forEach { t -> if (p.contains(t.key)) tuned[t] = p.getInt(t.key, t.default).coerceIn(t.range) }
        NOTIFY_KINDS.forEach { k -> if (!p.getBoolean("settings_notify_$k", true)) notifyOff[k] = true }
    }

    /** Every value, for the state dump. */
    fun snapshot(): Map<String, Any?> =
        mapOf("showSkipAdvice" to showSkipAdvice, "themeMode" to themeMode.name, "colors" to colorChoice, "pureBlack" to pureBlack, "navTabs" to navTabs, "avatar" to (avatarPath != null)) +
            mapOf("backgroundSync" to backgroundSync, "liveUpdates" to liveUpdates, "syncHours" to syncHours, "syncPaused" to (syncPaused != null), "notifyOff" to notifyOff.keys.sorted()) +
            mapOf("autoRefresh" to autoRefresh, "haptics" to haptics, "showBackgroundErrors" to showBackgroundErrors, "inspectTraffic" to inspectTraffic, "httpEngine" to httpEngine) + Tunable.entries.associate { it.key to tunable(it) }
}

/**
 * A number tuned in Settings → Developer (the Flutter app's `Tunable`), in [unit]s ("h", "min",
 * "s"), kept in [range]; [default] is the built-in value. [note] says what it changes.
 */
enum class Tunable(val key: String, val title: String, val unit: String, val default: Int, val range: IntRange, val note: String) {
    PageMaxAge("dev_page_max_age_h", "Page max age", "h", 6, 1..168, "Older pages fetch again on open and on return"),
    JustFetched("dev_just_fetched_min", "Just fetched", "min", 10, 0..120, "A page that finished this recently while away isn't fetched again"),
    CalendarMaxAge("dev_calendar_max_age_h", "Calendar max age", "h", 24, 1..720, "The academic calendar fetches again when this old"),
    SweepMaxAge("dev_sweep_max_age_h", "Background refresh age", "h", 12, 1..168, "Pages not on screen this old refresh in the open-app sweep"),
    SweepCooldown("dev_sweep_cooldown_min", "Background refresh gap", "min", 15, 0..1440, "At most one sweep in this long"),
    SweepQuick("dev_sweep_quick_s", "Quick fetch", "s", 2, 1..60, "A sweep starts only after a page fetch this fast"),
    SlowPage("dev_slow_page_s", "Background page timeout", "s", 5, 1..120, "A slower page stops the sweep"),
    VtopTimeout("dev_vtop_timeout_s", "VTOP request timeout", "s", 30, 5..180, "One VTOP request, from the next launch"),
    SessionReuse("dev_session_reuse_min", "VTOP session reuse", "min", 0, 0..10_080, "A saved session older than this signs in again at launch; 0: any age"),
    OtpPoll("dev_otp_poll_s", "Forwarded OTP check", "s", 2, 1..30, "How often the OTP prompt checks forwarded mail"),
    SessionAnswer("dev_session_answer_s", "Session request timeout", "s", 15, 3..60, "How long the phone tries to answer a browser's or AI app's session request"),
}

/** The background sync intervals on offer, in hours. */
val SYNC_HOURS = listOf(3, 6, 12, 24)
const val SYNC_HOURS_DEFAULT = 3

/** The kinds of change the inbox records and background sync can notify about. */
val NOTIFY_KINDS = listOf("attendance", "marks", "timetable", "exams", "grades")

enum class ThemeMode { System, Light, Dark }

/** Where the OTP prompt reads VTOP's code from. */
enum class OtpSource { Forwarding, Gmail }
