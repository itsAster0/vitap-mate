package vitapmate

import android.Manifest
import android.content.ContentProviderOperation
import android.content.ContentResolver
import android.content.ContentUris
import android.content.ContentValues
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.provider.CalendarContract.ACCOUNT_TYPE_LOCAL
import android.provider.CalendarContract.AUTHORITY
import android.provider.CalendarContract.CALLER_IS_SYNCADAPTER
import android.provider.CalendarContract.Calendars
import android.provider.CalendarContract.Events
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.ContextCompat
import com.composables.icons.lucide.CalendarSync as CalendarSyncIcon
import com.composables.icons.lucide.ExternalLink
import com.composables.icons.lucide.Lucide
import java.util.TimeZone
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.withContext
import vitapmate.core.CalendarDay
import vitapmate.core.CalendarMark
import vitapmate.core.Core
import vitapmate.data.AppCore
import vitapmate.data.CalendarDiff
import vitapmate.data.DAY_MS
import vitapmate.data.Foreground
import vitapmate.data.Settings
import vitapmate.data.SyncEvent
import vitapmate.data.attempt
import vitapmate.data.istDay
import vitapmate.features.timetable.meeting
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.observability.logAction
import vitapmate.ui.AppTheme
import vitapmate.ui.Palette
import vitapmate.ui.clock
import vitapmate.ui.rememberHaptics

/**
 * Settings → Calendar (off by default): the next [DAYS] days of classes, the dated exam papers and
 * the academic calendar's holidays, exam and no-class days, written from the cache into a local
 * "Vitap Mate" calendar on the phone (CalendarContract), so Google Calendar, watches and assistants
 * show them. Each event carries a stable key (SYNC_DATA1) and a sync writes only the difference
 * ([CalendarDiff]); turning it off deletes the calendar. Runs on turning on, on opening the app and
 * after each background sync. Flow `calendar.sync`, log src `calendar`.
 */
object CalendarSync {
    const val DAYS = 14
    private const val NAME = "Vitap Mate"
    private val PERMISSIONS = arrayOf(Manifest.permission.READ_CALENDAR, Manifest.permission.WRITE_CALENDAR)
    private val IST: TimeZone = TimeZone.getTimeZone("Asia/Kolkata")
    private val MARKED = setOf(CalendarMark.HOLIDAY, CalendarMark.EXAM, CalendarMark.LAB_FAT, CalendarMark.NO_CLASSES)
    private val PROJECTION = arrayOf(Events._ID, Events.SYNC_DATA1, Events.TITLE, Events.DTSTART, Events.DTEND, Events.ALL_DAY, Events.EVENT_LOCATION, Events.DESCRIPTION)

    /** One sync at a time, so two can't insert the same events. */
    private val lock = Mutex()

    fun granted(context: Context): Boolean = PERMISSIONS.all { ContextCompat.checkSelfPermission(context, it) == PackageManager.PERMISSION_GRANTED }

    /** Brings the calendar in step with the cache; off (and logged) once the permission is gone. */
    suspend fun sync(context: Context, flowName: String = "calendar.sync", unattended: Boolean = false) {
        if (!Settings.calendarSync) return
        val app = context.applicationContext
        flow(flowName, unattended) { f ->
            withContext(Dispatchers.IO) {
                lock.withLock {
                    if (!granted(app)) {
                        Settings.calendarSync = false
                        return@withLock Log.warn("calendar", "permission revoked, turned off", f)
                    }
                    val wanted = attempt { events(AppCore.await(), f, istDay(System.currentTimeMillis())) }
                        .onFailure { Log.warn("calendar", "cache unavailable: ${it::class.simpleName}", f) }.getOrNull() ?: return@withLock
                    val cr = app.contentResolver
                    val cal = calendarId(cr) ?: create(cr)
                    val rows = cr.query(Events.CONTENT_URI, PROJECTION, "${Events.CALENDAR_ID}=? AND ${Events.DELETED}=0", arrayOf("$cal"), null)?.use { c ->
                        buildMap {
                            while (c.moveToNext()) put(c.getLong(0), c.getString(1)?.let { SyncEvent(it, c.getString(2).orEmpty(), c.getLong(3), c.getLong(4), c.getInt(5) == 1, c.getString(6).orEmpty(), c.getString(7).orEmpty()) })
                        }
                    }.orEmpty()
                    // From today's midnight in India: earlier events stay as they were.
                    val d = CalendarDiff.of(rows, wanted, istDay(System.currentTimeMillis()) * DAY_MS - IST.rawOffset)
                    val uri = Events.CONTENT_URI.asAdapter()
                    val ops = ArrayList<ContentProviderOperation>()
                    d.insert.mapTo(ops) { ContentProviderOperation.newInsert(uri).withValues(values(cal, it)).build() }
                    d.update.mapTo(ops) { (id, e) -> ContentProviderOperation.newUpdate(ContentUris.withAppendedId(uri, id)).withValues(values(cal, e)).build() }
                    d.delete.mapTo(ops) { ContentProviderOperation.newDelete(ContentUris.withAppendedId(uri, it)).build() }
                    ops.chunked(200).forEach { cr.applyBatch(AUTHORITY, ArrayList(it)) }
                    Log.info("calendar", "synced", f, mapOf("inserted" to "${d.insert.size}", "updated" to "${d.update.size}", "deleted" to "${d.delete.size}", "events" to "${wanted.size}"))
                }
            }
        }
    }

    /** Deletes the app's calendar and its events (turning it off). */
    suspend fun remove(context: Context) = flow("calendar.remove") { f ->
        val app = context.applicationContext
        withContext(Dispatchers.IO) {
            lock.withLock {
                if (!granted(app)) return@withLock Log.warn("calendar", "no permission, calendar left", f)
                val cr = app.contentResolver
                val n = calendarId(cr)?.let { cr.delete(ContentUris.withAppendedId(Calendars.CONTENT_URI.asAdapter(), it), null, null) } ?: 0
                Log.info("calendar", "removed", f, mapOf("calendars" to "$n"))
            }
        }
    }

    /** Classes, exam papers and marked days, each with its stable key. */
    private fun events(c: Core, f: String, today: Int): List<SyncEvent> {
        val classes = c.cachedTimetable(f)?.let { c.timetableDays(f, today, DAYS) }.orEmpty().flatMap { d ->
            meeting(d).map { it.slot }.map { s ->
                SyncEvent(
                    "class.${d.day}.${s.start}.${s.code}", s.name, at(d.day, s.start), at(d.day, s.end),
                    location = join(" ", s.block, s.room), description = join(" · ", s.slot, s.faculty, s.code),
                )
            }
        }
        val exams = c.cachedExams(f)?.groups.orEmpty().flatMap { g ->
            g.papers.mapNotNull { p ->
                val day = p.day ?: return@mapNotNull null
                val about = join(" · ", p.reporting?.let { "Reporting ${clock(it, true)}" }, p.seat.ifEmpty { null }?.let { "Seat $it" }, p.seatNo.ifEmpty { null }?.let { "Seat no. $it" }, p.slot, p.code)
                val key = "exam.${g.name}.${p.key}"
                val title = "${g.name}: ${p.name}"
                val start = p.start
                if (start == null) SyncEvent(key, title, day * DAY_MS, (day + 1) * DAY_MS, true, p.venue, about)
                else SyncEvent(key, title, at(day, start), at(day, p.end ?: (start + 180)), false, p.venue, about)
            }
        }
        // A run of days with one title (a week of CATs) is one all-day event.
        val runs = mutableListOf<MutableList<CalendarDay>>()
        c.cachedCalendar(f)?.months.orEmpty().flatMap { it.days }.filter { it.mark in MARKED }.distinctBy { it.day }.sortedBy { it.day }.forEach { d ->
            val last = runs.lastOrNull()?.last()
            if (last != null && last.day + 1 == d.day && last.title == d.title) runs.last() += d else runs += mutableListOf(d)
        }
        val days = runs.map { r ->
            val first = r.first()
            SyncEvent("day.${first.day}", first.title.ifEmpty { first.label }, first.day * DAY_MS, (r.last().day + 1) * DAY_MS, true, description = first.label)
        }
        return classes + exams + days
    }

    /** Epoch ms of [minute] past midnight in India on [day]. */
    private fun at(day: Int, minute: Int): Long = day * DAY_MS + minute * 60_000L - IST.rawOffset

    private fun join(sep: String, vararg parts: String?) = parts.filterNot { it.isNullOrEmpty() }.joinToString(sep)

    private fun values(cal: Long, e: SyncEvent) = ContentValues().apply {
        put(Events.CALENDAR_ID, cal)
        put(Events.SYNC_DATA1, e.key)
        put(Events.TITLE, e.title)
        put(Events.DTSTART, e.start)
        put(Events.DTEND, e.end)
        put(Events.ALL_DAY, if (e.allDay) 1 else 0)
        // All-day events are UTC midnights, as CalendarContract wants them.
        put(Events.EVENT_TIMEZONE, if (e.allDay) "UTC" else IST.id)
        put(Events.EVENT_LOCATION, e.location)
        put(Events.DESCRIPTION, e.description)
        put(Events.AVAILABILITY, if (e.allDay) Events.AVAILABILITY_FREE else Events.AVAILABILITY_BUSY)
    }

    /** As the calendar's own (local) sync adapter: needed to create calendars and write SYNC_DATA1. */
    private fun Uri.asAdapter(): Uri = buildUpon()
        .appendQueryParameter(CALLER_IS_SYNCADAPTER, "true")
        .appendQueryParameter(Calendars.ACCOUNT_NAME, NAME)
        .appendQueryParameter(Calendars.ACCOUNT_TYPE, ACCOUNT_TYPE_LOCAL)
        .build()

    private fun calendarId(cr: ContentResolver): Long? = cr.query(
        Calendars.CONTENT_URI, arrayOf(Calendars._ID), "${Calendars.ACCOUNT_TYPE}=? AND ${Calendars.ACCOUNT_NAME}=?", arrayOf(ACCOUNT_TYPE_LOCAL, NAME), null,
    )?.use { if (it.moveToFirst()) it.getLong(0) else null }

    /** Read only for the student: the next sync would undo an edit. */
    private fun create(cr: ContentResolver): Long = ContentUris.parseId(
        cr.insert(
            Calendars.CONTENT_URI.asAdapter(),
            ContentValues().apply {
                put(Calendars.ACCOUNT_NAME, NAME)
                put(Calendars.ACCOUNT_TYPE, ACCOUNT_TYPE_LOCAL)
                put(Calendars.NAME, NAME)
                put(Calendars.CALENDAR_DISPLAY_NAME, NAME)
                put(Calendars.CALENDAR_COLOR, Palette.Default.accent.toArgb())
                put(Calendars.CALENDAR_ACCESS_LEVEL, Calendars.CAL_ACCESS_READ)
                put(Calendars.OWNER_ACCOUNT, NAME)
                put(Calendars.CALENDAR_TIME_ZONE, IST.id)
                put(Calendars.VISIBLE, 1)
                put(Calendars.SYNC_EVENTS, 1)
            },
        )!!,
    )

    /**
     * Settings → Calendar: the switch asks for calendar access first; refused, it stays off and a
     * row opens the app's system settings, where access can still be given.
     */
    @Composable
    fun Setting() {
        val context = LocalContext.current
        val scope = rememberCoroutineScope()
        val haptics = rememberHaptics()
        var refused by remember { mutableStateOf(false) }
        // Read again on return from the system setting.
        val allowed = remember(Foreground.resumes) { granted(context) }
        val turn = { on: Boolean ->
            Settings.calendarSync = on
            haptics.toggle(on)
            scope.launch { if (on) sync(context) else remove(context) }
            scope.logAction("settings.calendar_sync", "ui.settings", "calendar sync ${if (on) "on" else "off"}")
        }
        val ask = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) { result ->
            val ok = result.isNotEmpty() && result.values.all { it }
            scope.logAction("settings.calendar_permission", "ui.settings", "calendar permission", mapOf("granted" to "$ok"))
            refused = !ok
            if (ok) turn(true)
        }
        val set = { on: Boolean -> if (on && !granted(context)) ask.launch(PERMISSIONS) else turn(on) }
        ListGroup("Calendar", footer = "Into a \"Vitap Mate\" calendar on this phone, from what the app has saved. Turning it off deletes that calendar.") {
            GroupItem(
                "Add to my calendar",
                "settings.calendar_sync",
                Lucide.CalendarSyncIcon,
                if (refused && !allowed) "Stays off without calendar access" else "Classes for the next two weeks, exams and holidays, in Google Calendar and on your watch",
                onClick = { set(!Settings.calendarSync) },
                trailing = { ItemSwitch(Settings.calendarSync, { set(it) }) },
                on = Settings.calendarSync,
            )
            if (refused && !allowed) GroupItem(
                "Allow calendar access",
                "settings.calendar_sync.allow",
                Lucide.ExternalLink,
                "In Permissions, allow Calendar, then turn this on",
                onClick = { context.startActivity(Intent(android.provider.Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.fromParts("package", context.packageName, null))) },
                tone = AppTheme.tones.warning,
            )
        }
    }
}
