package vitapmate.data

import android.content.Context
import android.content.SharedPreferences
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.core.content.edit
import kotlin.reflect.KProperty
import vitapmate.core.ChangeKind

/** The display sizes offered, in percent of the app's own fit (100); 0 follows the phone's display size. */
val DISPLAY_SIZES = listOf(0, 90, 100, 110)

/** App settings (SharedPreferences "settings", the Flutter app's keys) as Compose-observable state. */
object Settings {
    @Volatile private var prefs: SharedPreferences? = null
    private val all = mutableListOf<Pref<*>>()
    private val tuned = mutableStateMapOf<Tunable, Int>()
    private val notifyOff = mutableStateMapOf<ChangeKind, Boolean>()

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

    private fun ids(key: String) = pref<Set<String>>(key, emptySet(), { k, d -> getStringSet(k, d)?.toSet() ?: d }, { k, v -> putStringSet(k, v) })

    /** An enum saved as its lowercase name. */
    private fun <T : Enum<*>?> choice(key: String, default: T, entries: List<T & Any>) =
        pref(key, default, { k, d -> entries.firstOrNull { it.name.lowercase() == getString(k, null) } ?: d }, { k, v -> putString(k, v?.name?.lowercase()) })

    /** Whether attendance shows how many classes can be skipped. */
    var showSkipAdvice by flag("settings_show_skip_advice", false)

    /** Whether app-computed totals, GPAs, projections, cross-page joins and calculators are shown. */
    var calculatedInsights by flag("settings_calculated_insights", false)

    /** Whether skip advice is active: requires [calculatedInsights] to be on too. */
    val effectiveSkipAdvice: Boolean get() = calculatedInsights && showSkipAdvice

    /**
     * Whether Connected apps' note "storage stayed on because you saved your password" was read
     * (or the student chose storage there): it shows once.
     */
    var storageNoteSeen by flag("settings_storage_note_seen", false)

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

    /** The timetable's view; the Flutter app's key and values (anything else, like the removed "daily" view, opens the agenda). */
    var timetableView by choice("settings_timetable_view_mode", TimetableViewMode.Agenda, TimetableViewMode.entries)

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

    /** Settings → Appearance → Display size: one of [DISPLAY_SIZES]. */
    var displaySize by pref("settings_display_size", 100, { k, d -> getInt(k, d).takeIf { it in DISPLAY_SIZES } ?: d }, { k, v -> putInt(k, v) })

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
     * (HTTP/3 where the server offers it); otherwise the platform's HttpURLConnection.
     */
    var httpEngine by flag("dev_http_engine", true)

    /**
     * Background sync (WorkManager, every [syncHours]): off by default, and offered only while OTP
     * auto-fill works, since nobody can type a code in the background.
     */
    var backgroundSync by flag("settings_background_sync", false)

    /** Live updates (Android 16+): the class on now and outing requests as promoted notifications; off by default. */
    var liveUpdates by flag("settings_live_updates", false)

    /** In class: Do Not Disturb while a class or exam is on (ClassMode); off by default. */
    var classMode by flag("settings_class_mode", false)

    /** Assistant (Android 16+): the saved timetable and attendance for Android's app functions; off by default. */
    var assistant by flag("settings_assistant", false)

    /** Calendar: classes, exams and the academic calendar in a calendar on the phone (app's CalendarSync); off by default. */
    var calendarSync by flag("settings_calendar_sync", false)

    /** The setup page after the first semester pick (app's Onboarding) was seen; shown once per install. */
    var onboarded by flag("settings_onboarded", false)

    /** A semester was picked the first time, and the setup page that follows isn't done yet. */
    var onboardingPending by flag("settings_onboarding_pending", false)

    /**
     * The student (their username, as [AccountWatch] names them) whose things Kotlin keeps:
     * the rest of these settings, their photo and Gmail grant. Another one signing in lets go
     * of them; null signed out, or from before this was kept.
     */
    var student by text("settings_student")

    /** Semester ids whose Wrapped was opened. */
    var wrappedSeen by ids("settings_wrapped_seen")

    /** Semester ids whose Wrapped share images hide grades and marks. */
    var wrappedHideGrades by ids("settings_wrapped_hide_grades")

    /** Wrapped share images leave out the first name. */
    var wrappedHideName by flag("settings_wrapped_hide_name", false)

    /** Wrapped share images leave out hostel times and outings. */
    var wrappedHideHostel by flag("settings_wrapped_hide_hostel", false)

    /** Hours between background syncs, one of [SYNC_HOURS]. */
    var syncHours by pref("settings_sync_hours", SYNC_HOURS_DEFAULT, { k, d -> getInt(k, d).takeIf { it in SYNC_HOURS } ?: d }, { k, v -> putInt(k, v) })

    /**
     * How background sync stands: paused when a background run couldn't finish the OTP (no code
     * coming only after two runs in a row), until the next good run. The reason's id is saved
     * where older apps kept its text; the miss count beside it.
     */
    var syncHealth by pref(
        "settings_sync_paused",
        SyncHealth.OK,
        { k, _ -> SyncHealth.parse(getString(k, null), getInt(SYNC_MISSES, 0)) },
        { k, v ->
            when (v) {
                is SyncHealth.Active -> remove(k).putInt(SYNC_MISSES, v.otpMisses)
                is SyncHealth.Paused -> putString(k, v.reason.id).remove(SYNC_MISSES)
            }
        },
    )

    private const val SYNC_MISSES = "settings_sync_otp_misses"

    /** The course part background sync last checked for new files ([VtopSync]). */
    var syncCourse by text("settings_sync_course")

    /**
     * Forgets what was set up for the student who signed out or was replaced: the OTP source
     * (their Gmail or address), background sync built on it, and where sync left off. The caller
     * disconnects Gmail and re-applies the sync job.
     */
    fun forgetStudent() {
        otpSource = null
        backgroundSync = false
        syncHealth = SyncHealth.OK
        syncCourse = null
        wrappedSeen = emptySet()
        wrappedHideGrades = emptySet()
        wrappedHideName = false
        wrappedHideHostel = false
        storageNoteSeen = false
        student = null
    }

    /** Whether background sync notifies about changes of [kind]. */
    fun notifies(kind: ChangeKind): Boolean = notifyOff[kind] != true

    fun setNotifies(kind: ChangeKind, on: Boolean) {
        if (on) notifyOff.remove(kind) else notifyOff[kind] = true
        prefs?.edit { putBoolean(notifyKey(kind), on) }
    }

    /** `settings_notify_attendance`, ...: the key older apps saved under too. */
    private fun notifyKey(kind: ChangeKind) = "settings_notify_${kind.name.lowercase()}"

    /** [t]'s value, in its unit. */
    fun tunable(t: Tunable): Int = tuned[t] ?: t.default

    /** Saves [value] (kept in [Tunable.range]) for [t]; null goes back to its default. */
    fun setTunable(t: Tunable, value: Int?) {
        val v = value?.coerceIn(t.range)
        if (v == null || v == t.default) tuned.remove(t) else tuned[t] = v
        prefs?.edit { if (v == null || v == t.default) remove(t.key) else putInt(t.key, v) }
    }

    /**
     * Reads the saved values, once: every entry point (the activity, FCM, WorkManager, widgets,
     * the tile) calls it first, on whatever thread it runs, and returns with them read.
     */
    @Synchronized
    fun load(context: Context) {
        if (prefs != null) return
        val p = context.getSharedPreferences("settings", Context.MODE_PRIVATE)
        // Older apps counted missed codes in their own file.
        val old = context.getSharedPreferences("sync", Context.MODE_PRIVATE)
        if (old.contains("otp_misses")) {
            p.edit { putInt(SYNC_MISSES, old.getInt("otp_misses", 0)) }
            old.edit { remove("otp_misses") }
        }
        all.forEach { it.load(p) }
        Tunable.entries.forEach { t -> if (p.contains(t.key)) tuned[t] = p.getInt(t.key, t.default).coerceIn(t.range) }
        ChangeKind.entries.forEach { k -> if (!p.getBoolean(notifyKey(k), true)) notifyOff[k] = true }
        // Set last: once [prefs] is there, every value has been read.
        prefs = p
    }

    /** Every value, for the state dump. */
    fun snapshot(): Map<String, Any?> =
        mapOf("calculatedInsights" to calculatedInsights, "showSkipAdvice" to showSkipAdvice, "themeMode" to themeMode.name, "timetableView" to timetableView.name, "colors" to colorChoice, "pureBlack" to pureBlack, "displaySize" to displaySize, "navTabs" to navTabs, "avatar" to (avatarPath != null)) +
            mapOf("backgroundSync" to backgroundSync, "liveUpdates" to liveUpdates, "classMode" to classMode, "assistant" to assistant, "calendarSync" to calendarSync, "syncHours" to syncHours, "syncPaused" to (syncHealth is SyncHealth.Paused), "notifyOff" to notifyOff.keys.map { it.name.lowercase() }.sorted()) +
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
    FetchLanes("dev_fetch_lanes", "Pages at once", "pages", 2, 1..4, "Page fetches that run at the same time; the rest wait, the page on screen first"),
    SlowFetch("dev_slow_fetch_s", "Slow link", "s", 4, 1..60, "When recent page fetches take this long, one runs at a time and the sweep waits"),
    QueueDrop("dev_queue_drop_s", "Waiting fetch limit", "s", 60, 5..600, "An automatic fetch for a page left behind is dropped after waiting this long"),
    VtopTimeout("dev_vtop_timeout_s", "VTOP request timeout", "s", 30, 5..180, "One VTOP request, from the next launch"),
    SessionReuse("dev_session_reuse_min", "VTOP session reuse", "min", 0, 0..10_080, "A saved session older than this signs in again at launch; 0: any age"),
    OtpPoll("dev_otp_poll_s", "OTP check without the live socket", "s", 2, 1..30, "How often the OTP prompt checks Gmail, or forwarded mail when the server can't push it"),
    SessionAnswer("dev_session_answer_s", "Session request timeout", "s", 15, 3..60, "How long the phone tries to answer a browser's or AI app's session request"),
    CookieAsk("dev_cookie_ask_s", "Cookie consent wait", "s", 60, 10..300, "How long the 'send your VTOP cookie?' dialog waits before it counts as Cancel"),
    SyncBiometricDays("dev_sync_bio_days", "Biometric days per sync", "days", 3, 0..7, "Recent days not final in the cache, newest first; 0 turns it off"),
    CampusFetchGap("dev_campus_gap_s", "Campus fetch gap", "s", 3, 0..30, "Pause between biometric days in the Wrapped fetch"),
    CampusFetchAtOnce("dev_campus_at_once", "Campus fetch at once", "days", 2, 1..3, "Biometric days the Wrapped fetch asks VTOP for at the same time"),
    WrappedRefetch("dev_wrapped_refetch_h", "Wrapped refetch age", "h", 12, 1..168, "Pages with nothing posted yet fetch again for Wrapped when this old"),
    ProofTimeout("dev_proof_bg_timeout_s", "Background proof timeout", "s", 90, 20..300, "The account proof after sign-in gives up after this long and tries again at the next open"),
    ProofOnCellular("dev_proof_cellular", "Background proof on mobile data", "on", 0, 0..1, "1: the account proof after sign-in (about 1.5 MB) may run on mobile data too"),
}

/** The background sync intervals on offer, in hours. */
val SYNC_HOURS = listOf(3, 6, 12, 24)
const val SYNC_HOURS_DEFAULT = 3

enum class ThemeMode { System, Light, Dark }

enum class TimetableViewMode { Agenda, Weekly }

/** Where the OTP prompt reads VTOP's code from. */
enum class OtpSource { Forwarding, Gmail }
