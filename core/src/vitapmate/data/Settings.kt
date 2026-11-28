package vitapmate.data

import android.content.Context
import android.os.Handler
import android.os.Looper
import androidx.compose.runtime.mutableStateOf
import androidx.core.content.edit
import vitapmate.core.ChangeKind
import vitapmate.core.PrefValue
import vitapmate.core.SettingChange
import vitapmate.core.SettingsListener
import vitapmate.core.SettingsStore
import vitapmate.core.TunableInfo
import vitapmate.core.TunableKey
import vitapmate.core.settingsPrefKeys
import vitapmate.core.tunables
import vitapmate.observability.Log
import vitapmate.core.OtpSource as RustOtpSource
import vitapmate.core.Settings as RustSettings
import vitapmate.core.ThemeMode as RustThemeMode
import vitapmate.core.TimetableViewMode as RustTimetableViewMode

/** The display sizes offered, in percent of the app's own fit (100); 0 follows the phone's display size. */
val DISPLAY_SIZES = listOf(0, 90, 100, 110)

/**
 * App settings as Compose-observable state, mirroring Rust's [SettingsStore] (`files/settings.json`).
 * Rust is the single owner; this object holds a view of the last snapshot and forwards every write.
 */
object Settings {
    @Volatile private var store: SettingsStore? = null
    @Volatile private var tunableMap: Map<TunableKey, TunableInfo>? = null
    private val state = mutableStateOf(defaultRustSettings())

    const val WALLPAPER = "wallpaper"

    private fun applyChange(change: SettingChange, fallback: (RustSettings) -> RustSettings) {
        val s = store
        state.value = if (s != null) s.set(change) else fallback(state.value)
    }

    /** Whether attendance shows how many classes can be skipped. */
    var showSkipAdvice: Boolean
        get() = state.value.showSkipAdvice
        set(value) = applyChange(SettingChange.ShowSkipAdvice(value)) { it.copy(showSkipAdvice = value) }

    /** Whether app-computed totals, GPAs, projections, cross-page joins and calculators are shown. */
    var calculatedInsights: Boolean
        get() = state.value.calculatedInsights
        set(value) = applyChange(SettingChange.CalculatedInsights(value)) { it.copy(calculatedInsights = value) }

    /** Whether skip advice is active: requires [calculatedInsights] to be on too. */
    val effectiveSkipAdvice: Boolean get() = calculatedInsights && showSkipAdvice

    /**
     * Whether Connected apps' note "storage stayed on because you saved your password" was read
     * (or the student chose storage there): it shows once.
     */
    var storageNoteSeen: Boolean
        get() = state.value.storageNoteSeen
        set(value) = applyChange(SettingChange.StorageNoteSeen(value)) { it.copy(storageNoteSeen = value) }

    /**
     * Which one fills in VTOP's OTP: one at a time. Set when one is set up or connected, or chosen
     * with "Use this for OTPs"; null before either is. The other stays set up, on standby.
     */
    var otpSource: OtpSource?
        get() = state.value.otpSource?.toKotlin()
        set(value) = applyChange(SettingChange.OtpSource(value?.toRust())) { it.copy(otpSource = value?.toRust()) }

    /** Haptic feedback on taps, toggles, refreshes and results (the system's touch feedback setting still applies). */
    var haptics: Boolean
        get() = state.value.haptics
        set(value) = applyChange(SettingChange.Haptics(value)) { it.copy(haptics = value) }

    /**
     * The Flutter app's "Auto Refresh": pages fetch on their own (on open, when old, on return to
     * the app, the open-app sweep). Off, only a page with nothing saved fetches by itself.
     */
    var autoRefresh: Boolean
        get() = state.value.autoRefresh
        set(value) = applyChange(SettingChange.AutoRefresh(value)) { it.copy(autoRefresh = value) }

    /** Failures of work the student didn't ask for (auto refresh, the sweep) show as toasts too; off, they only log. */
    var showBackgroundErrors: Boolean
        get() = state.value.showBackgroundErrors
        set(value) = applyChange(SettingChange.ShowBackgroundErrors(value)) { it.copy(showBackgroundErrors = value) }

    /** The timetable's view; the Flutter app's key and values (anything else, like the removed "daily" view, opens the agenda). */
    var timetableView: TimetableViewMode
        get() = state.value.timetableView.toKotlin()
        set(value) = applyChange(SettingChange.TimetableView(value.toRust())) { it.copy(timetableView = value.toRust()) }

    /** Light, dark, or as the phone is set. */
    var themeMode: ThemeMode
        get() = state.value.themeMode.toKotlin()
        set(value) = applyChange(SettingChange.ThemeMode(value.toRust())) { it.copy(themeMode = value.toRust()) }

    /** The colours: [WALLPAPER], a palette's name (the design module's `Palette`), or null for the default. */
    var colorChoice: String?
        get() = state.value.colorChoice
        set(value) = applyChange(SettingChange.ColorChoice(value)) { it.copy(colorChoice = value) }

    /** Black backgrounds in dark mode (OLED screens). */
    var pureBlack: Boolean
        get() = state.value.pureBlack
        set(value) = applyChange(SettingChange.PureBlack(value)) { it.copy(pureBlack = value) }

    /** Settings → Appearance → Display size: one of [DISPLAY_SIZES]. */
    var displaySize: Int
        get() = state.value.displaySize
        set(value) {
            val valid = value.takeIf { it in DISPLAY_SIZES } ?: 100
            applyChange(SettingChange.DisplaySize(valid)) { it.copy(displaySize = valid) }
        }

    /** The student's chosen avatar photo (a file in the app's private files), or null for the initial. */
    var avatarPath: String?
        get() = state.value.avatarPath
        set(value) = applyChange(SettingChange.AvatarPath(value)) { it.copy(avatarPath = value) }

    /**
     * The nav bar's tabs, feature ids in order (rules in [NavTabs]: at most 5, always "more").
     * Ids of features that no longer exist may be in it; the app skips them.
     */
    val navTabs: List<String> get() = state.value.navTabs

    fun setNavTabs(ids: List<String>) {
        val next = NavTabs.normalize(ids)
        applyChange(SettingChange.NavTabs(next)) { it.copy(navTabs = next) }
    }

    /** Pins [id]; false when the bar is full and nothing changed. */
    fun addNavTab(id: String): Boolean {
        val current = navTabs
        val ok = id in current || current.size < NavTabs.MAX
        setNavTabs(NavTabs.add(current, id))
        return ok
    }

    fun removeNavTab(id: String) = setNavTabs(NavTabs.remove(navTabs, id))

    fun moveNavTab(id: String, to: Int) = setNavTabs(NavTabs.move(navTabs, id, to))

    /**
     * Developer listed in Settings, as on Android: turned on by tapping the version 7 times,
     * off from the switch at the top of Developer. Kept across launches.
     */
    var developerEnabled: Boolean
        get() = state.value.developerEnabled
        set(value) = applyChange(SettingChange.DeveloperEnabled(value)) { it.copy(developerEnabled = value) }

    /** Developer → Inspect Traffic: from the next launch, VTOP requests use the Wi-Fi proxy and trust user CAs. */
    var inspectTraffic: Boolean
        get() = state.value.inspectTraffic
        set(value) = applyChange(SettingChange.InspectTraffic(value)) { it.copy(inspectTraffic = value) }

    /**
     * Developer → HttpEngine, on by default: from the next launch, on Android 14+ (where the
     * phone has HttpEngine) and with Inspect Traffic off, requests go through [VtopHttpEngine]
     * (HTTP/3 where the server offers it); otherwise the platform's HttpURLConnection.
     */
    var httpEngine: Boolean
        get() = state.value.httpEngine
        set(value) = applyChange(SettingChange.HttpEngine(value)) { it.copy(httpEngine = value) }

    /**
     * Background sync (WorkManager, every [syncHours]): off by default, and offered only while OTP
     * auto-fill works, since nobody can type a code in the background.
     */
    var backgroundSync: Boolean
        get() = state.value.backgroundSync
        set(value) = applyChange(SettingChange.BackgroundSync(value)) { it.copy(backgroundSync = value) }

    /** Live updates (Android 16+): the class on now and outing requests as promoted notifications; off by default. */
    var liveUpdates: Boolean
        get() = state.value.liveUpdates
        set(value) = applyChange(SettingChange.LiveUpdates(value)) { it.copy(liveUpdates = value) }

    /** In class: Do Not Disturb while a class or exam is on (ClassMode); off by default. */
    var classMode: Boolean
        get() = state.value.classMode
        set(value) = applyChange(SettingChange.ClassMode(value)) { it.copy(classMode = value) }

    /** Assistant (Android 16+): the saved timetable and attendance for Android's app functions; off by default. */
    var assistant: Boolean
        get() = state.value.assistant
        set(value) = applyChange(SettingChange.Assistant(value)) { it.copy(assistant = value) }

    /** Calendar: classes, exams and the academic calendar in a calendar on the phone (app's CalendarSync); off by default. */
    var calendarSync: Boolean
        get() = state.value.calendarSync
        set(value) = applyChange(SettingChange.CalendarSync(value)) { it.copy(calendarSync = value) }

    /** The setup page after the first semester pick (app's Onboarding) was seen; shown once per install. */
    var onboarded: Boolean
        get() = state.value.onboarded
        set(value) = applyChange(SettingChange.Onboarded(value)) { it.copy(onboarded = value) }

    /** A semester was picked the first time, and the setup page that follows isn't done yet. */
    var onboardingPending: Boolean
        get() = state.value.onboardingPending
        set(value) = applyChange(SettingChange.OnboardingPending(value)) { it.copy(onboardingPending = value) }

    /**
     * The student (their username, as [AccountWatch] names them) whose things Kotlin keeps:
     * the rest of these settings, their photo and Gmail grant. Another one signing in lets go
     * of them; null signed out, or from before this was kept.
     */
    var student: String?
        get() = state.value.student
        set(value) = applyChange(SettingChange.Student(value)) { it.copy(student = value) }

    /** Semester ids whose Wrapped was opened. */
    var wrappedSeen: Set<String>
        get() = state.value.wrappedSeen.toSet()
        set(value) {
            val sorted = value.sorted()
            applyChange(SettingChange.WrappedSeen(sorted)) { it.copy(wrappedSeen = sorted) }
        }

    /** Semester ids whose Wrapped share images hide grades and marks. */
    var wrappedHideGrades: Set<String>
        get() = state.value.wrappedHideGrades.toSet()
        set(value) {
            val sorted = value.sorted()
            applyChange(SettingChange.WrappedHideGrades(sorted)) { it.copy(wrappedHideGrades = sorted) }
        }

    /** Wrapped share images leave out the first name. */
    var wrappedHideName: Boolean
        get() = state.value.wrappedHideName
        set(value) = applyChange(SettingChange.WrappedHideName(value)) { it.copy(wrappedHideName = value) }

    /** Wrapped share images leave out hostel times and outings. */
    var wrappedHideHostel: Boolean
        get() = state.value.wrappedHideHostel
        set(value) = applyChange(SettingChange.WrappedHideHostel(value)) { it.copy(wrappedHideHostel = value) }

    /** Hours between background syncs, one of [SYNC_HOURS]. */
    var syncHours: Int
        get() = state.value.syncHours
        set(value) {
            val valid = value.takeIf { it in SYNC_HOURS } ?: SYNC_HOURS_DEFAULT
            applyChange(SettingChange.SyncHours(valid)) { it.copy(syncHours = valid) }
        }

    /**
     * Forgets what was set up for the student who signed out or was replaced: the OTP source
     * (their Gmail or address), background sync built on it, and where sync left off. The caller
     * disconnects Gmail and re-applies the sync job.
     */
    fun forgetStudent() = applyChange(SettingChange.ForgetStudent) {
        it.copy(
            otpSource = null,
            backgroundSync = false,
            syncPaused = null,
            syncOtpMisses = 0,
            syncCourse = null,
            wrappedSeen = emptyList(),
            wrappedHideGrades = emptyList(),
            wrappedHideName = false,
            wrappedHideHostel = false,
            storageNoteSeen = false,
            student = null,
        )
    }

    /** Whether background sync notifies about changes of [kind]. */
    fun notifies(kind: ChangeKind): Boolean = kind !in state.value.notifyOff

    fun setNotifies(kind: ChangeKind, on: Boolean) = applyChange(SettingChange.Notifies(kind, on)) { current ->
        val next = if (on) current.notifyOff - kind else (current.notifyOff + kind).distinct()
        current.copy(notifyOff = next)
    }

    /** [t]'s value, in its unit. */
    fun tunable(t: Tunable): Int = state.value.tuned[t.key] ?: t.default

    /** Saves [value] (kept in [Tunable.range]) for [t]; null goes back to its default. */
    fun setTunable(t: Tunable, value: Int?) = applyChange(SettingChange.Tunable(t.rustKey, value)) { current ->
        val clamped = value?.coerceIn(t.range)
        val next = current.tuned.toMutableMap()
        if (clamped == null || clamped == t.default) next.remove(t.key) else next[t.key] = clamped
        current.copy(tuned = next)
    }

    internal fun tunableInfo(key: TunableKey): TunableInfo? {
        tunableMap?.let { return it[key] }
        val loaded = runCatching { tunables().associateBy { it.key } }.getOrNull() ?: return null
        tunableMap = loaded
        return loaded[key]
    }

    /**
     * Opens Rust's [SettingsStore] synchronously and imports legacy SharedPreferences once if
     * `settings.json` does not exist yet. Safe from any thread; later calls do nothing.
     */
    @Synchronized
    fun load(context: Context) {
        if (store != null) return
        val app = context.applicationContext
        val s = SettingsStore.open(app.filesDir.path)
        tunableMap = tunables().associateBy { it.key }
        if (s.needsImport()) {
            val p = app.getSharedPreferences("settings", Context.MODE_PRIVATE)
            val old = app.getSharedPreferences("sync", Context.MODE_PRIVATE)
            val map = buildMap<String, PrefValue> {
                p.all.forEach { (key, raw) -> toPrefValue(raw)?.let { put(key, it) } }
                if (old.contains("otp_misses")) {
                    put("settings_sync_otp_misses", PrefValue.Int(old.getInt("otp_misses", 0)))
                }
            }
            runCatching { s.import(map) }
                .onSuccess { imported ->
                    state.value = imported
                    // Clear only keys owned by Settings so other screens sharing "settings" prefs keep theirs.
                    val ownedKeys = settingsPrefKeys()
                    p.edit { ownedKeys.forEach(::remove) }
                    if (old.contains("otp_misses")) old.edit { remove("otp_misses") }
                }
                .onFailure { e ->
                    Log.warn("settings", "import failed: ${e::class.simpleName}")
                    state.value = s.get()
                }
        } else {
            state.value = s.get()
        }
        val main = Handler(Looper.getMainLooper())
        s.setListener(object : SettingsListener {
            override fun onSettings(settings: RustSettings) {
                if (Looper.myLooper() == Looper.getMainLooper()) {
                    state.value = settings
                } else {
                    main.post { state.value = settings }
                }
            }
        })
        // Set last: once [store] is visible, every value has been read.
        store = s
    }

    /** Every value, for the state dump. */
    fun snapshot(): Map<String, Any?> =
        mapOf("calculatedInsights" to calculatedInsights, "showSkipAdvice" to showSkipAdvice, "themeMode" to themeMode.name, "timetableView" to timetableView.name, "colors" to colorChoice, "pureBlack" to pureBlack, "displaySize" to displaySize, "navTabs" to navTabs, "avatar" to (avatarPath != null)) +
            mapOf("backgroundSync" to backgroundSync, "liveUpdates" to liveUpdates, "classMode" to classMode, "assistant" to assistant, "calendarSync" to calendarSync, "syncHours" to syncHours, "syncPaused" to (SyncHealthState.pausedText != null), "notifyOff" to state.value.notifyOff.map { it.name.lowercase() }.sorted()) +
            mapOf("autoRefresh" to autoRefresh, "haptics" to haptics, "showBackgroundErrors" to showBackgroundErrors, "inspectTraffic" to inspectTraffic, "httpEngine" to httpEngine) + Tunable.entries.associate { it.key to tunable(it) }
}

private fun toPrefValue(raw: Any?): PrefValue? = when (raw) {
    is Boolean -> PrefValue.Bool(raw)
    is Int -> PrefValue.Int(raw)
    is Long -> PrefValue.Long(raw)
    is Float -> PrefValue.Float(raw)
    is String -> PrefValue.Str(raw)
    is Set<*> -> PrefValue.StrSet(raw.filterIsInstance<String>())
    else -> null
}

private fun defaultRustSettings() = RustSettings(
    showSkipAdvice = false,
    calculatedInsights = false,
    storageNoteSeen = false,
    otpSource = null,
    haptics = true,
    autoRefresh = true,
    showBackgroundErrors = false,
    timetableView = RustTimetableViewMode.AGENDA,
    themeMode = RustThemeMode.SYSTEM,
    colorChoice = null,
    pureBlack = false,
    displaySize = 100,
    avatarPath = null,
    navTabs = NavTabs.DEFAULT,
    developerEnabled = false,
    inspectTraffic = false,
    httpEngine = true,
    backgroundSync = false,
    liveUpdates = false,
    classMode = false,
    assistant = false,
    calendarSync = false,
    onboarded = false,
    onboardingPending = false,
    student = null,
    wrappedSeen = emptyList(),
    wrappedHideGrades = emptyList(),
    wrappedHideName = false,
    wrappedHideHostel = false,
    syncHours = SYNC_HOURS_DEFAULT,
    syncPaused = null,
    syncOtpMisses = 0,
    syncCourse = null,
    notifyOff = emptyList(),
    tuned = emptyMap(),
)

/**
 * A number tuned in Settings → Developer (the Flutter app's `Tunable`). Metadata (key, title,
 * unit, default, range, note) is owned by Rust (`tunables()`); the fallback numbers exist only so
 * host JVM unit tests without `libvtop_ffi.so` can still read defaults.
 */
enum class Tunable(
    internal val rustKey: TunableKey,
    private val fallbackKey: String,
    private val fallbackDefault: Int,
    private val fallbackRange: IntRange,
) {
    PageMaxAge(TunableKey.PAGE_MAX_AGE, "dev_page_max_age_h", 6, 1..168),
    JustFetched(TunableKey.JUST_FETCHED, "dev_just_fetched_min", 10, 0..120),
    CalendarMaxAge(TunableKey.CALENDAR_MAX_AGE, "dev_calendar_max_age_h", 24, 1..720),
    SweepMaxAge(TunableKey.SWEEP_MAX_AGE, "dev_sweep_max_age_h", 12, 1..168),
    SweepCooldown(TunableKey.SWEEP_COOLDOWN, "dev_sweep_cooldown_min", 15, 0..1440),
    SweepQuick(TunableKey.SWEEP_QUICK, "dev_sweep_quick_s", 2, 1..60),
    SlowPage(TunableKey.SLOW_PAGE, "dev_slow_page_s", 5, 1..120),
    FetchLanes(TunableKey.FETCH_LANES, "dev_fetch_lanes", 2, 1..4),
    SlowFetch(TunableKey.SLOW_FETCH, "dev_slow_fetch_s", 4, 1..60),
    QueueDrop(TunableKey.QUEUE_DROP, "dev_queue_drop_s", 60, 5..600),
    VtopTimeout(TunableKey.VTOP_TIMEOUT, "dev_vtop_timeout_s", 30, 5..180),
    SessionReuse(TunableKey.SESSION_REUSE, "dev_session_reuse_min", 0, 0..10_080),
    OtpPoll(TunableKey.OTP_POLL, "dev_otp_poll_s", 2, 1..30),
    SessionAnswer(TunableKey.SESSION_ANSWER, "dev_session_answer_s", 15, 3..60),
    CookieAsk(TunableKey.COOKIE_ASK, "dev_cookie_ask_s", 60, 10..300),
    SyncBiometricDays(TunableKey.SYNC_BIOMETRIC_DAYS, "dev_sync_bio_days", 3, 0..7),
    CampusFetchGap(TunableKey.CAMPUS_FETCH_GAP, "dev_campus_gap_s", 3, 0..30),
    CampusFetchAtOnce(TunableKey.CAMPUS_FETCH_AT_ONCE, "dev_campus_at_once", 2, 1..3),
    WrappedRefetch(TunableKey.WRAPPED_REFETCH, "dev_wrapped_refetch_h", 12, 1..168),
    ProofTimeout(TunableKey.PROOF_TIMEOUT, "dev_proof_bg_timeout_s", 90, 20..300),
    ProofOnCellular(TunableKey.PROOF_ON_CELLULAR, "dev_proof_cellular", 0, 0..1);

    private val info: TunableInfo? get() = Settings.tunableInfo(rustKey)

    val key: String get() = info?.prefKey ?: fallbackKey
    val title: String get() = info?.title ?: name
    val unit: String get() = info?.unit ?: ""
    val default: Int get() = info?.default ?: fallbackDefault
    val range: IntRange get() = info?.let { it.min..it.max } ?: fallbackRange
    val note: String get() = info?.note ?: ""
}

/** The background sync intervals on offer, in hours. */
val SYNC_HOURS = listOf(3, 6, 12, 24)
const val SYNC_HOURS_DEFAULT = 3

enum class ThemeMode { System, Light, Dark }

enum class TimetableViewMode { Agenda, Weekly }

/** Where the OTP prompt reads VTOP's code from. */
enum class OtpSource { Forwarding, Gmail }

private fun RustOtpSource.toKotlin(): OtpSource = when (this) {
    RustOtpSource.FORWARDING -> OtpSource.Forwarding
    RustOtpSource.GMAIL -> OtpSource.Gmail
}

private fun OtpSource.toRust(): RustOtpSource = when (this) {
    OtpSource.Forwarding -> RustOtpSource.FORWARDING
    OtpSource.Gmail -> RustOtpSource.GMAIL
}

private fun RustThemeMode.toKotlin(): ThemeMode = when (this) {
    RustThemeMode.SYSTEM -> ThemeMode.System
    RustThemeMode.LIGHT -> ThemeMode.Light
    RustThemeMode.DARK -> ThemeMode.Dark
}

private fun ThemeMode.toRust(): RustThemeMode = when (this) {
    ThemeMode.System -> RustThemeMode.SYSTEM
    ThemeMode.Light -> RustThemeMode.LIGHT
    ThemeMode.Dark -> RustThemeMode.DARK
}

private fun RustTimetableViewMode.toKotlin(): TimetableViewMode = when (this) {
    RustTimetableViewMode.AGENDA -> TimetableViewMode.Agenda
    RustTimetableViewMode.WEEKLY -> TimetableViewMode.Weekly
}

private fun TimetableViewMode.toRust(): RustTimetableViewMode = when (this) {
    TimetableViewMode.Agenda -> RustTimetableViewMode.AGENDA
    TimetableViewMode.Weekly -> RustTimetableViewMode.WEEKLY
}
