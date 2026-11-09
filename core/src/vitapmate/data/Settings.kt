package vitapmate.data

import android.content.Context
import android.content.SharedPreferences
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.core.content.edit


/** App settings (SharedPreferences "settings", the Flutter app's keys) as Compose-observable state. */
object Settings {
    private var prefs: SharedPreferences? = null
    private val skipAdvice = mutableStateOf(false)
    private val theme = mutableStateOf(ThemeMode.System)
    private val colors = mutableStateOf<String?>(null)
    private val black = mutableStateOf(false)
    private val tabs = mutableStateOf(NavTabs.DEFAULT)
    private val avatar = mutableStateOf<String?>(null)
    private val otp = mutableStateOf<OtpSource?>(null)
    private val auto = mutableStateOf(true)
    private val haptic = mutableStateOf(true)
    private val backgroundErrors = mutableStateOf(false)
    private val devAlways = mutableStateOf(false)
    private val inspect = mutableStateOf(false)
    private val engine = mutableStateOf(false)
    private val tuned = mutableStateMapOf<Tunable, Int>()

    /** Whether attendance shows how many classes can be skipped. */
    var showSkipAdvice: Boolean
        get() = skipAdvice.value
        set(value) {
            skipAdvice.value = value
            prefs?.edit()?.putBoolean("settings_show_skip_advice", value)?.apply()
        }

    /**
     * Which one fills in VTOP's OTP: one at a time. Set when one is set up or connected, or chosen
     * with "Use this for OTPs"; null before either is. The other stays set up, on standby.
     */
    var otpSource: OtpSource?
        get() = otp.value
        set(value) {
            otp.value = value
            prefs?.edit()?.putString("settings_otp_source", value?.name?.lowercase())?.apply()
        }

    /** Haptic feedback on taps, toggles, refreshes and results (the system's touch feedback setting still applies). */
    var haptics: Boolean
        get() = haptic.value
        set(value) {
            haptic.value = value
            prefs?.edit()?.putBoolean("settings_haptics", value)?.apply()
        }

    /**
     * The Flutter app's "Auto Refresh": pages fetch on their own (on open, when old, on return to
     * the app, the open-app sweep). Off, only a page with nothing saved fetches by itself.
     */
    var autoRefresh: Boolean
        get() = auto.value
        set(value) {
            auto.value = value
            prefs?.edit()?.putBoolean("settings_auto_refresh", value)?.apply()
        }

    /** Failures of work the student didn't ask for (auto refresh, the sweep) show as toasts too; off, they only log. */
    var showBackgroundErrors: Boolean
        get() = backgroundErrors.value
        set(value) {
            backgroundErrors.value = value
            prefs?.edit()?.putBoolean("settings_show_background_errors", value)?.apply()
        }

    /** Light, dark, or as the phone is set. */
    var themeMode: ThemeMode
        get() = theme.value
        set(value) {
            theme.value = value
            prefs?.edit()?.putString("settings_theme_mode", value.name.lowercase())?.apply()
        }

    /** The colours: [WALLPAPER], a palette's name (the design module's `Palette`), or null for the default. */
    var colorChoice: String?
        get() = colors.value
        set(value) {
            colors.value = value
            prefs?.edit()?.putString("settings_colors", value)?.apply()
        }

    const val WALLPAPER = "wallpaper"

    /** Black backgrounds in dark mode (OLED screens). */
    var pureBlack: Boolean
        get() = black.value
        set(value) {
            black.value = value
            prefs?.edit()?.putBoolean("settings_pure_black", value)?.apply()
        }

    /** The student's chosen avatar photo (a file in the app's private files), or null for the initial. */
    var avatarPath: String?
        get() = avatar.value
        set(value) {
            avatar.value = value
            prefs?.edit{putString("settings_avatar_path", value)}
        }

    /**
     * The nav bar's tabs, feature ids in order (rules in [NavTabs]: at most 4, always "more").
     * Ids of features that no longer exist may be in it; the app skips them.
     */
    val navTabs: List<String> get() = tabs.value

    fun setNavTabs(ids: List<String>) = saveTabs(NavTabs.normalize(ids))

    /** Pins [id]; false when the bar is full (4) and nothing changed. */
    fun addNavTab(id: String): Boolean = (id in navTabs || navTabs.size < NavTabs.MAX).also { saveTabs(NavTabs.add(navTabs, id)) }

    fun removeNavTab(id: String) = saveTabs(NavTabs.remove(navTabs, id))

    fun moveNavTab(id: String, to: Int) = saveTabs(NavTabs.move(navTabs, id, to))

    private fun saveTabs(value: List<String>) {
        tabs.value = value
        prefs?.edit()?.putString("settings_nav_tabs", NavTabs.encode(value))?.apply()
    }

    /**
     * Developer listed in Settings, as on Android: turned on by tapping the version 7 times,
     * off from the switch at the top of Developer. Kept across launches.
     */
    var developerEnabled: Boolean
        get() = devAlways.value
        set(value) {
            devAlways.value = value
            prefs?.edit()?.putBoolean("dev_always_show", value)?.apply()
        }

    /** Developer → Inspect Traffic: from the next launch, VTOP requests use the Wi-Fi proxy and trust user CAs. */
    var inspectTraffic: Boolean
        get() = inspect.value
        set(value) {
            inspect.value = value
            prefs?.edit()?.putBoolean("dev_inspect_traffic", value)?.apply()
        }

    /**
     * Developer → HttpEngine, on by default: from the next launch, on Android 14+ (where the
     * phone has HttpEngine) and with Inspect Traffic off, requests go through [VtopHttpEngine]
     * (HTTP/3 where the server offers it); otherwise plain OkHttp.
     */
    var httpEngine: Boolean
        get() = engine.value
        set(value) {
            engine.value = value
            prefs?.edit()?.putBoolean("dev_http_engine", value)?.apply()
        }

    /** [t]'s value, in its unit. */
    fun tunable(t: Tunable): Int = tuned[t] ?: t.default

    /** Saves [value] (kept in [Tunable.range]) for [t]; null goes back to its default. */
    fun setTunable(t: Tunable, value: Int?) {
        val v = value?.coerceIn(t.range)
        if (v == null || v == t.default) tuned.remove(t) else tuned[t] = v
        prefs?.edit()?.apply { if (v == null || v == t.default) remove(t.key) else putInt(t.key, v) }?.apply()
    }

    /** Reads the saved values; call once at start. */
    fun load(context: Context) {
        val p = context.getSharedPreferences("settings", Context.MODE_PRIVATE)
        prefs = p
        skipAdvice.value = p.getBoolean("settings_show_skip_advice", false)
        auto.value = p.getBoolean("settings_auto_refresh", true)
        haptic.value = p.getBoolean("settings_haptics", true)
        backgroundErrors.value = p.getBoolean("settings_show_background_errors", false)
        theme.value = ThemeMode.entries.firstOrNull { it.name.lowercase() == p.getString("settings_theme_mode", null) } ?: ThemeMode.System
        black.value = p.getBoolean("settings_pure_black", false)
        // The old wallpaper switch carries over.
        colors.value = p.getString("settings_colors", null) ?: WALLPAPER.takeIf { p.getBoolean("settings_dynamic_color", false) }
        tabs.value = NavTabs.decode(p.getString("settings_nav_tabs", null))
        avatar.value = p.getString("settings_avatar_path", null)
        otp.value = OtpSource.entries.firstOrNull { it.name.lowercase() == p.getString("settings_otp_source", null) }
        devAlways.value = p.getBoolean("dev_always_show", false)
        inspect.value = p.getBoolean("dev_inspect_traffic", false)
        engine.value = p.getBoolean("dev_http_engine", true)
        Tunable.entries.forEach { t -> if (p.contains(t.key)) tuned[t] = p.getInt(t.key, t.default).coerceIn(t.range) }
    }

    /** Every value, for the state dump. */
    fun snapshot(): Map<String, Any?> =
        mapOf("showSkipAdvice" to showSkipAdvice, "themeMode" to themeMode.name, "colors" to colorChoice, "pureBlack" to pureBlack, "navTabs" to navTabs, "avatar" to (avatarPath != null)) +
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
}

enum class ThemeMode { System, Light, Dark }

/** Where the OTP prompt reads VTOP's code from. */
enum class OtpSource { Forwarding, Gmail }
