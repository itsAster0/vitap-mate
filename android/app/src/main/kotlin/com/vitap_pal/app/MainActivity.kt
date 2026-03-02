package com.vitap_pal.app

import android.Manifest
import android.content.ContentValues
import android.content.Intent
import android.content.pm.PackageManager
import android.provider.CalendarContract
import androidx.core.content.ContextCompat
import io.flutter.embedding.android.FlutterFragmentActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodCall
import io.flutter.plugin.common.MethodChannel
import java.util.Calendar
import java.util.TimeZone

class MainActivity : FlutterFragmentActivity() {
    companion object {
        private const val CHANNEL = "vitapmate/google_calendar_sync"
        private const val SYNC_TAG = "[VitapMateTimetable]"
    }

    private data class CalendarTarget(
        val id: Long,
        val displayName: String,
        val isPrimary: Boolean,
        val accountName: String
    )

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "syncTimetableToGoogleCalendar" -> syncTimetable(call, result)
                    "clearAllSyncedEvents" -> clearAllSyncedEvents(call, result)
                    "getGoogleAccounts" -> getGoogleAccounts(result)
                    "getAvailableCalendarApps" -> getAvailableCalendarApps(result)
                    "openCalendarApp" -> openCalendarApp(call, result)
                    else -> result.notImplemented()
                }
            }
    }

    private fun syncTimetable(call: MethodCall, result: MethodChannel.Result) {
        if (!hasCalendarPermission()) {
            result.error("permission_denied", "Calendar permission not granted", null)
            return
        }

        val accountName = call.argument<String>("accountName")
        val titleTemplate = call.argument<String>("titleTemplate") ?: "{name}"
        val descriptionTemplate = call.argument<String>("descriptionTemplate")
            ?: "Course: {courseCode}\nType: {courseType}\nSlot: {slot}\nFaculty: {faculty}"
        val locationTemplate = call.argument<String>("locationTemplate") ?: "{block}-{roomNo}"
        val calendar = resolveWritableGoogleCalendar(accountName) ?: run {
            result.error("google_calendar_not_found", "No writable Google calendar found", null)
            return
        }

        val semesterId = call.argument<String>("semesterId")?.trim().orEmpty()
        if (semesterId.isEmpty()) {
            result.error("invalid_semester_id", "Semester ID is required for sync", null)
            return
        }
        val reminderMinutes = (call.argument<Number>("reminderMinutes")?.toInt() ?: 10).coerceAtLeast(0)
        val startEpochMs = call.argument<Number>("startEpochMs")?.toLong()
        val untilEpochMs = call.argument<Number>("untilEpochMs")?.toLong()
        val rawSlots = call.argument<List<*>>("slots") ?: emptyList<Any?>()

        removeSyncedEvents(
            calendar.id,
            semesterId,
            startEpochMs
        )

        var created = 0
        for (item in rawSlots) {
            val slot = item as? Map<*, *> ?: continue
            if (
                createRecurringEvent(
                    calendar.id,
                    slot,
                    semesterId,
                    startEpochMs,
                    untilEpochMs,
                    reminderMinutes,
                    titleTemplate,
                    descriptionTemplate,
                    locationTemplate
                )
            ) {
                created++
            }
        }

        result.success(
            mapOf(
                "created" to created,
                "calendarName" to calendar.displayName
            )
        )
    }

    private fun hasCalendarPermission(): Boolean {
        val readGranted = ContextCompat.checkSelfPermission(
            this,
            Manifest.permission.READ_CALENDAR
        ) == PackageManager.PERMISSION_GRANTED
        val writeGranted = ContextCompat.checkSelfPermission(
            this,
            Manifest.permission.WRITE_CALENDAR
        ) == PackageManager.PERMISSION_GRANTED
        return readGranted && writeGranted
    }

    private fun resolveWritableGoogleCalendar(accountName: String?): CalendarTarget? {
        val googleCalendars = getWritableGoogleCalendars(accountName)
        if (googleCalendars.isEmpty()) return null

        val primary = googleCalendars.firstOrNull { it.isPrimary }
        return primary ?: googleCalendars.firstOrNull()
    }

    private fun getWritableGoogleCalendars(accountName: String? = null): List<CalendarTarget> {
        val googleCalendars = mutableListOf<CalendarTarget>()

        val projection = arrayOf(
            CalendarContract.Calendars._ID,
            CalendarContract.Calendars.CALENDAR_DISPLAY_NAME,
            CalendarContract.Calendars.ACCOUNT_TYPE,
            CalendarContract.Calendars.CALENDAR_ACCESS_LEVEL,
            CalendarContract.Calendars.IS_PRIMARY,
            CalendarContract.Calendars.ACCOUNT_NAME
        )

        val minAccessLevel = CalendarContract.Calendars.CAL_ACCESS_CONTRIBUTOR
        val cursor = contentResolver.query(
            CalendarContract.Calendars.CONTENT_URI,
            projection,
            "${CalendarContract.Calendars.SYNC_EVENTS}=1 AND ${CalendarContract.Calendars.CALENDAR_ACCESS_LEVEL}>=?",
            arrayOf(minAccessLevel.toString()),
            null,
            null
        )

        cursor?.use {
            val idCol = it.getColumnIndexOrThrow(CalendarContract.Calendars._ID)
            val displayNameCol = it.getColumnIndexOrThrow(CalendarContract.Calendars.CALENDAR_DISPLAY_NAME)
            val accountTypeCol = it.getColumnIndexOrThrow(CalendarContract.Calendars.ACCOUNT_TYPE)
            val isPrimaryCol = it.getColumnIndexOrThrow(CalendarContract.Calendars.IS_PRIMARY)
            val accountNameCol = it.getColumnIndexOrThrow(CalendarContract.Calendars.ACCOUNT_NAME)

            while (it.moveToNext()) {
                val id = it.getLong(idCol)
                val displayName = it.getString(displayNameCol) ?: "Google Calendar"
                val accountType = it.getString(accountTypeCol) ?: ""
                val isPrimary = (it.getInt(isPrimaryCol) == 1)
                val account = it.getString(accountNameCol) ?: ""

                if (accountType == "com.google" && (accountName == null || account == accountName)) {
                    val target = CalendarTarget(
                        id = id,
                        displayName = displayName,
                        isPrimary = isPrimary,
                        accountName = account
                    )
                    googleCalendars.add(target)
                }
            }
        }

        return googleCalendars
    }

    private fun getGoogleAccounts(result: MethodChannel.Result) {
        if (!hasCalendarPermission()) {
            result.success(emptyList<String>())
            return
        }
        val accounts = getWritableGoogleCalendars()
            .map { it.accountName.trim() }
            .filter { it.isNotEmpty() }
            .toSet()
            .toList()
            .sorted()
        result.success(accounts)
    }

    private fun clearAllSyncedEvents(call: MethodCall, result: MethodChannel.Result) {
        if (!hasCalendarPermission()) {
            result.error("permission_denied", "Calendar permission not granted", null)
            return
        }

        val accountName = call.argument<String>("accountName")
        val semesterId = call.argument<String>("semesterId")?.trim().orEmpty()
        if (semesterId.isEmpty()) {
            result.error("invalid_semester_id", "Semester ID is required for clear", null)
            return
        }
        val calendar = resolveWritableGoogleCalendar(accountName) ?: run {
            result.error("google_calendar_not_found", "No writable Google calendar found", null)
            return
        }

        val deleted = clearTaggedEvents(calendar.id, semesterId)
        result.success(
            mapOf(
                "deleted" to deleted,
                "calendarName" to calendar.displayName
            )
        )
    }

    private fun removeSyncedEvents(
        calendarId: Long,
        semesterId: String,
        startEpochMs: Long?
    ): Int {
        val semesterToken = semesterTag(semesterId)
        val (selection, args) =
            if (startEpochMs != null) {
                // Resync scope: remove semester-tagged events from selected start date onward.
                // Recurring series are removed too so future occurrences are recreated with latest data.
                Pair(
                    "${CalendarContract.Events.CALENDAR_ID}=? AND " +
                        "${CalendarContract.Events.DESCRIPTION} LIKE ? AND " +
                        "${CalendarContract.Events.DESCRIPTION} LIKE ? AND (" +
                        "${CalendarContract.Events.DTSTART}>=? OR " +
                        "${CalendarContract.Events.RRULE} IS NOT NULL" +
                        ")",
                    arrayOf(
                        calendarId.toString(),
                        "%$SYNC_TAG%",
                        "%$semesterToken%",
                        startEpochMs.toString()
                    )
                )
            } else {
                Pair(
                    "${CalendarContract.Events.CALENDAR_ID}=? AND " +
                        "${CalendarContract.Events.DESCRIPTION} LIKE ? AND " +
                        "${CalendarContract.Events.DESCRIPTION} LIKE ?",
                    arrayOf(
                        calendarId.toString(),
                        "%$SYNC_TAG%",
                        "%$semesterToken%"
                    )
                )
            }
        return contentResolver.delete(CalendarContract.Events.CONTENT_URI, selection, args)
    }

    private fun clearTaggedEvents(calendarId: Long, semesterId: String): Int {
        val semesterToken = semesterTag(semesterId)
        val selection =
            "${CalendarContract.Events.CALENDAR_ID}=? AND " +
                "${CalendarContract.Events.DESCRIPTION} LIKE ? AND " +
                "${CalendarContract.Events.DESCRIPTION} LIKE ?"
        val args = arrayOf(
            calendarId.toString(),
            "%$SYNC_TAG%",
            "%$semesterToken%"
        )
        return contentResolver.delete(CalendarContract.Events.CONTENT_URI, selection, args)
    }

    private fun createRecurringEvent(
        calendarId: Long,
        slot: Map<*, *>,
        semesterId: String,
        startEpochMs: Long?,
        untilEpochMs: Long?,
        reminderMinutes: Int,
        titleTemplate: String,
        descriptionTemplate: String,
        locationTemplate: String
    ): Boolean {
        val serial = slot["serial"]?.toString() ?: return false
        if (serial == "-1") return false

        val day = slot["day"]?.toString() ?: return false
        val byDay = toByDay(day) ?: return false

        val baseMillis = startEpochMs ?: System.currentTimeMillis()
        var startMillis = nextOccurrenceMillisFromBase(day, slot["startTime"]?.toString(), baseMillis) ?: return false
        var endMillis = nextOccurrenceMillisFromBase(day, slot["endTime"]?.toString(), startMillis) ?: return false

        if (untilEpochMs != null && startMillis > untilEpochMs) return false
        if (endMillis <= startMillis) {
            endMillis = startMillis + 30 * 60 * 1000
        }

        val title = renderTemplate(
            titleTemplate,
            slot
        ).ifBlank { slot["name"]?.toString()?.ifBlank { "Class" } ?: "Class" }
        val location = renderTemplate(
            locationTemplate,
            slot
        ).ifBlank { buildLocation(slot["block"]?.toString(), slot["roomNo"]?.toString()) }
        val description = renderTemplate(
            descriptionTemplate,
            slot
        ).ifBlank { buildDescription(slot, semesterId) }
        val trackedDescription = buildTrackedDescription(description, semesterId)
        val rrule = if (untilEpochMs != null) {
            "FREQ=WEEKLY;BYDAY=$byDay;UNTIL=${toRruleUntil(untilEpochMs)}"
        } else {
            "FREQ=WEEKLY;BYDAY=$byDay"
        }

        val values = ContentValues().apply {
            put(CalendarContract.Events.CALENDAR_ID, calendarId)
            put(CalendarContract.Events.TITLE, title)
            put(CalendarContract.Events.DESCRIPTION, trackedDescription)
            put(CalendarContract.Events.EVENT_LOCATION, location)
            put(CalendarContract.Events.DTSTART, startMillis)
            put(CalendarContract.Events.DTEND, endMillis)
            put(CalendarContract.Events.EVENT_TIMEZONE, TimeZone.getDefault().id)
            put(CalendarContract.Events.RRULE, rrule)
            put(CalendarContract.Events.AVAILABILITY, CalendarContract.Events.AVAILABILITY_BUSY)
            put(CalendarContract.Events.STATUS, CalendarContract.Events.STATUS_CONFIRMED)
            put(CalendarContract.Events.HAS_ALARM, if (reminderMinutes > 0) 1 else 0)
        }

        val uri = contentResolver.insert(CalendarContract.Events.CONTENT_URI, values)
        if (uri == null) return false
        val eventId = uri.lastPathSegment?.toLongOrNull() ?: return true

        contentResolver.delete(
            CalendarContract.Reminders.CONTENT_URI,
            "${CalendarContract.Reminders.EVENT_ID}=?",
            arrayOf(eventId.toString())
        )

        if (reminderMinutes > 0) {
            val reminder = ContentValues().apply {
                put(CalendarContract.Reminders.EVENT_ID, eventId)
                put(CalendarContract.Reminders.MINUTES, reminderMinutes)
                put(CalendarContract.Reminders.METHOD, CalendarContract.Reminders.METHOD_ALERT)
            }
            contentResolver.insert(CalendarContract.Reminders.CONTENT_URI, reminder)
        } else {
            val noAlarm = ContentValues().apply {
                put(CalendarContract.Events.HAS_ALARM, 0)
            }
            contentResolver.update(
                CalendarContract.Events.CONTENT_URI,
                noAlarm,
                "${CalendarContract.Events._ID}=?",
                arrayOf(eventId.toString())
            )
        }
        return true
    }

    private fun buildDescription(slot: Map<*, *>, semesterId: String): String {
        val code = slot["courseCode"]?.toString() ?: ""
        val courseType = slot["courseType"]?.toString() ?: ""
        val faculty = slot["faculty"]?.toString() ?: ""
        val slotName = slot["slot"]?.toString() ?: ""

        return buildString {
            append("Course: $code\nType: $courseType\nSlot: $slotName\nFaculty: $faculty")
            append("\n${semesterTag(semesterId)}")
            append("\n$SYNC_TAG")
        }
    }

    private fun buildTrackedDescription(description: String, semesterId: String): String {
        var out = description.trim()
        val semesterToken = semesterTag(semesterId)
        if (!out.contains(semesterToken)) {
            out = if (out.isEmpty()) semesterToken else "$out\n$semesterToken"
        }
        if (!out.contains(SYNC_TAG)) {
            out = if (out.isEmpty()) SYNC_TAG else "$out\n$SYNC_TAG"
        }
        return out
    }

    private fun renderTemplate(template: String, slot: Map<*, *>): String {
        val replacements = mapOf(
            "serial" to (slot["serial"]?.toString() ?: ""),
            "day" to (slot["day"]?.toString() ?: ""),
            "slot" to (slot["slot"]?.toString() ?: ""),
            "courseCode" to (slot["courseCode"]?.toString() ?: ""),
            "courseType" to (slot["courseType"]?.toString() ?: ""),
            "roomNo" to (slot["roomNo"]?.toString() ?: ""),
            "block" to (slot["block"]?.toString() ?: ""),
            "startTime" to (slot["startTime"]?.toString() ?: ""),
            "endTime" to (slot["endTime"]?.toString() ?: ""),
            "name" to (slot["name"]?.toString() ?: ""),
            "faculty" to (slot["faculty"]?.toString() ?: "")
        )
        var out = template
        for ((key, value) in replacements) {
            out = out.replace("{$key}", value)
        }
        return out
    }

    private fun semesterTag(semesterId: String): String {
        return "SEM:${semesterId.trim()}"
    }

    private fun buildLocation(block: String?, roomNo: String?): String {
        val b = block?.trim().orEmpty()
        val r = roomNo?.trim().orEmpty()
        return when {
            b.isEmpty() && r.isEmpty() -> "VIT-AP"
            b.isEmpty() -> r
            r.isEmpty() -> b
            else -> "$b-$r"
        }
    }

    private fun nextOccurrenceMillisFromBase(dayCode: String, hhmm: String?, baseMillis: Long): Long? {
        val targetDay = toCalendarDay(dayCode) ?: return null
        val parts = hhmm?.split(":") ?: return null
        val hour = parts.getOrNull(0)?.toIntOrNull() ?: return null
        val minute = parts.getOrNull(1)?.toIntOrNull() ?: 0

        val now = Calendar.getInstance().apply {
            timeInMillis = baseMillis
        }
        val cal = Calendar.getInstance().apply {
            timeInMillis = baseMillis
            set(Calendar.SECOND, 0)
            set(Calendar.MILLISECOND, 0)
            set(Calendar.HOUR_OF_DAY, hour)
            set(Calendar.MINUTE, minute)
        }

        while (cal.get(Calendar.DAY_OF_WEEK) != targetDay) {
            cal.add(Calendar.DAY_OF_MONTH, 1)
        }
        if (cal.timeInMillis < now.timeInMillis) {
            cal.add(Calendar.DAY_OF_MONTH, 7)
        }
        return cal.timeInMillis
    }

    private fun toCalendarDay(dayCode: String): Int? {
        return when (dayCode) {
            "MON" -> Calendar.MONDAY
            "TUE" -> Calendar.TUESDAY
            "WED" -> Calendar.WEDNESDAY
            "THU" -> Calendar.THURSDAY
            "FRI" -> Calendar.FRIDAY
            "SAT" -> Calendar.SATURDAY
            "SUN" -> Calendar.SUNDAY
            else -> null
        }
    }

    private fun toByDay(dayCode: String): String? {
        return when (dayCode) {
            "MON" -> "MO"
            "TUE" -> "TU"
            "WED" -> "WE"
            "THU" -> "TH"
            "FRI" -> "FR"
            "SAT" -> "SA"
            "SUN" -> "SU"
            else -> null
        }
    }

    private fun getAvailableCalendarApps(result: MethodChannel.Result) {
        val apps = mutableListOf<Map<String, String>>()
        val seenPackages = mutableSetOf<String>()
        val calendarIntent = Intent(Intent.ACTION_MAIN).apply {
            addCategory(Intent.CATEGORY_APP_CALENDAR)
        }

        val resolved = packageManager.queryIntentActivities(calendarIntent, 0)
        for (resolveInfo in resolved) {
            val packageName = resolveInfo.activityInfo?.packageName ?: continue
            if (seenPackages.contains(packageName)) continue

            val label = resolveInfo.loadLabel(packageManager)?.toString() ?: packageName
            apps.add(mapOf("packageName" to packageName, "label" to label))
            seenPackages.add(packageName)
        }

        val googlePackage = "com.google.android.calendar"
        val googleLaunch = packageManager.getLaunchIntentForPackage(googlePackage)
        if (googleLaunch != null && !seenPackages.contains(googlePackage)) {
            apps.add(0, mapOf("packageName" to googlePackage, "label" to "Google Calendar"))
        } else {
            apps.sortByDescending { if (it["packageName"] == googlePackage) 1 else 0 }
        }

        result.success(apps)
    }

    private fun openCalendarApp(call: MethodCall, result: MethodChannel.Result) {
        val packageName = call.argument<String>("packageName")
        if (packageName.isNullOrBlank()) {
            result.success(false)
            return
        }

        val launchIntent = packageManager.getLaunchIntentForPackage(packageName)
        if (launchIntent != null) {
            launchIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            startActivity(launchIntent)
            result.success(true)
            return
        }

        result.success(false)
    }

    private fun toRruleUntil(epochMillis: Long): String {
        val utcCal = Calendar.getInstance(TimeZone.getTimeZone("UTC")).apply {
            timeInMillis = epochMillis
            set(Calendar.HOUR_OF_DAY, 23)
            set(Calendar.MINUTE, 59)
            set(Calendar.SECOND, 59)
            set(Calendar.MILLISECOND, 0)
        }

        val year = utcCal.get(Calendar.YEAR).toString().padStart(4, '0')
        val month = (utcCal.get(Calendar.MONTH) + 1).toString().padStart(2, '0')
        val day = utcCal.get(Calendar.DAY_OF_MONTH).toString().padStart(2, '0')
        val hour = utcCal.get(Calendar.HOUR_OF_DAY).toString().padStart(2, '0')
        val minute = utcCal.get(Calendar.MINUTE).toString().padStart(2, '0')
        val second = utcCal.get(Calendar.SECOND).toString().padStart(2, '0')
        return "${year}${month}${day}T${hour}${minute}${second}Z"
    }

}
