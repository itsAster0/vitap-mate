package com.vitap_pal.app

import android.Manifest
import android.content.ContentValues
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
        private const val TAG_PREFIX = "vitapmate-"
    }

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, CHANNEL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "getWritableCalendars" -> getWritableCalendars(result)
                    "syncTimetableRecurring" -> syncTimetableRecurring(call, result)
                    "clearAllSyncedEvents" -> clearAllSyncedEvents(call, result)
                    else -> result.notImplemented()
                }
            }
    }

    private fun getWritableCalendars(result: MethodChannel.Result) {
        if (!hasCalendarPermission()) {
            result.success(emptyList<Map<String, String>>())
            return
        }

        val projection = arrayOf(
            CalendarContract.Calendars._ID,
            CalendarContract.Calendars.CALENDAR_DISPLAY_NAME,
            CalendarContract.Calendars.CALENDAR_ACCESS_LEVEL,
            CalendarContract.Calendars.ACCOUNT_NAME
        )
        val minAccessLevel = CalendarContract.Calendars.CAL_ACCESS_CONTRIBUTOR
        val cursor = contentResolver.query(
            CalendarContract.Calendars.CONTENT_URI,
            projection,
            "${CalendarContract.Calendars.SYNC_EVENTS}=1 AND ${CalendarContract.Calendars.CALENDAR_ACCESS_LEVEL}>=?",
            arrayOf(minAccessLevel.toString()),
            null
        )

        val items = mutableListOf<Map<String, String>>()
        cursor?.use {
            val idCol = it.getColumnIndexOrThrow(CalendarContract.Calendars._ID)
            val nameCol = it.getColumnIndexOrThrow(CalendarContract.Calendars.CALENDAR_DISPLAY_NAME)
            val accountCol = it.getColumnIndexOrThrow(CalendarContract.Calendars.ACCOUNT_NAME)
            while (it.moveToNext()) {
                val id = it.getLong(idCol).toString()
                val name = it.getString(nameCol) ?: "Calendar"
                val account = it.getString(accountCol) ?: ""
                items.add(
                    mapOf(
                        "id" to id,
                        "name" to name,
                        "accountName" to account
                    )
                )
            }
        }
        result.success(items)
    }

    private fun syncTimetableRecurring(call: MethodCall, result: MethodChannel.Result) {
        if (!hasCalendarPermission()) {
            result.error("permission_denied", "Calendar permission not granted", null)
            return
        }

        val calendarId = call.argument<String>("calendarId")?.toLongOrNull()
        if (calendarId == null) {
            result.error("invalid_calendar_id", "Calendar ID is required for sync", null)
            return
        }
        val semesterId = call.argument<String>("semesterId")?.trim().orEmpty()
        if (semesterId.isEmpty()) {
            result.error("invalid_semester_id", "Semester ID is required for sync", null)
            return
        }
        val startEpochMs = call.argument<Number>("startEpochMs")?.toLong()
        if (startEpochMs == null) {
            result.error("invalid_start_date", "Start date is required", null)
            return
        }

        val untilEpochMs = call.argument<Number>("untilEpochMs")?.toLong()
        val reminderMinutes = (call.argument<Number>("reminderMinutes")?.toInt() ?: 10).coerceAtLeast(0)
        val titleTemplate = call.argument<String>("titleTemplate") ?: "{name}"
        val descriptionTemplate = call.argument<String>("descriptionTemplate")
            ?: "Course: {courseCode}\nType: {courseType}\nSlot: {slot}\nFaculty: {faculty}"
        val locationTemplate = call.argument<String>("locationTemplate") ?: "{block}-{roomNo}"
        val rawSlots = call.argument<List<*>>("slots") ?: emptyList<Any?>()

        val newVersion = getNextVersion(calendarId, semesterId)
        closeOlderVersions(calendarId, semesterId, newVersion, startEpochMs)

        var created = 0
        for (item in rawSlots) {
            val slot = item as? Map<*, *> ?: continue
            if (
                createRecurringEvent(
                    calendarId = calendarId,
                    slot = slot,
                    semesterId = semesterId,
                    version = newVersion,
                    startEpochMs = startEpochMs,
                    untilEpochMs = untilEpochMs,
                    reminderMinutes = reminderMinutes,
                    titleTemplate = titleTemplate,
                    descriptionTemplate = descriptionTemplate,
                    locationTemplate = locationTemplate
                )
            ) {
                created++
            }
        }

        result.success(
            mapOf(
                "created" to created,
                "version" to newVersion,
                "calendarName" to getCalendarDisplayName(calendarId)
            )
        )
    }

    private fun clearAllSyncedEvents(call: MethodCall, result: MethodChannel.Result) {
        if (!hasCalendarPermission()) {
            result.error("permission_denied", "Calendar permission not granted", null)
            return
        }

        val calendarId = call.argument<String>("calendarId")?.toLongOrNull()
        if (calendarId == null) {
            result.error("invalid_calendar_id", "Calendar ID is required for clear", null)
            return
        }
        val semesterId = call.argument<String>("semesterId")?.trim().orEmpty()
        if (semesterId.isEmpty()) {
            result.error("invalid_semester_id", "Semester ID is required for clear", null)
            return
        }

        val deleted = clearTaggedEvents(calendarId, semesterId)
        result.success(
            mapOf(
                "deleted" to deleted,
                "calendarName" to getCalendarDisplayName(calendarId)
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

    private fun getCalendarDisplayName(calendarId: Long): String {
        val cursor = contentResolver.query(
            CalendarContract.Calendars.CONTENT_URI,
            arrayOf(CalendarContract.Calendars.CALENDAR_DISPLAY_NAME),
            "${CalendarContract.Calendars._ID}=?",
            arrayOf(calendarId.toString()),
            null
        )
        cursor?.use {
            if (it.moveToFirst()) {
                return it.getString(0) ?: "Calendar"
            }
        }
        return "Calendar"
    }

    private fun getNextVersion(calendarId: Long, semesterId: String): Int {
        val semesterTag = semesterTag(semesterId)
        val selection =
            "${CalendarContract.Events.CALENDAR_ID}=? AND " +
                "${CalendarContract.Events.DESCRIPTION} LIKE ? AND " +
                "${CalendarContract.Events.DESCRIPTION} LIKE ?"
        val args = arrayOf(
            calendarId.toString(),
            "%$SYNC_TAG%",
            "%$semesterTag%"
        )

        var maxVersion = 0
        val cursor = contentResolver.query(
            CalendarContract.Events.CONTENT_URI,
            arrayOf(CalendarContract.Events.DESCRIPTION),
            selection,
            args,
            null
        )
        cursor?.use {
            while (it.moveToNext()) {
                val description = it.getString(0) ?: continue
                val version = extractVersion(description) ?: continue
                if (version > maxVersion) {
                    maxVersion = version
                }
            }
        }
        return maxVersion + 1
    }

    private fun closeOlderVersions(
        calendarId: Long,
        semesterId: String,
        newVersion: Int,
        startEpochMs: Long
    ) {
        val semesterTag = semesterTag(semesterId)
        val selection =
            "${CalendarContract.Events.CALENDAR_ID}=? AND " +
                "${CalendarContract.Events.DESCRIPTION} LIKE ? AND " +
                "${CalendarContract.Events.DESCRIPTION} LIKE ?"
        val args = arrayOf(
            calendarId.toString(),
            "%$SYNC_TAG%",
            "%$semesterTag%"
        )
        val cutoffStart = startOfDay(startEpochMs)
        val previousDayEnd = cutoffStart - 1000L

        val cursor = contentResolver.query(
            CalendarContract.Events.CONTENT_URI,
            arrayOf(
                CalendarContract.Events._ID,
                CalendarContract.Events.DTSTART,
                CalendarContract.Events.RRULE,
                CalendarContract.Events.DESCRIPTION
            ),
            selection,
            args,
            null
        )
        cursor?.use {
            while (it.moveToNext()) {
                val eventId = it.getLong(0)
                val dtStart = it.getLong(1)
                val rrule = it.getString(2)
                val description = it.getString(3) ?: ""
                val version = extractVersion(description) ?: continue
                if (version >= newVersion) continue

                if (dtStart >= cutoffStart) {
                    contentResolver.delete(
                        CalendarContract.Events.CONTENT_URI,
                        "${CalendarContract.Events._ID}=?",
                        arrayOf(eventId.toString())
                    )
                    continue
                }

                if (!rrule.isNullOrBlank()) {
                    val updated = ContentValues().apply {
                        put(CalendarContract.Events.RRULE, upsertUntil(rrule, previousDayEnd))
                    }
                    contentResolver.update(
                        CalendarContract.Events.CONTENT_URI,
                        updated,
                        "${CalendarContract.Events._ID}=?",
                        arrayOf(eventId.toString())
                    )
                }
            }
        }
    }

    private fun clearTaggedEvents(calendarId: Long, semesterId: String): Int {
        val semesterTag = semesterTag(semesterId)
        val selection =
            "${CalendarContract.Events.CALENDAR_ID}=? AND " +
                "${CalendarContract.Events.DESCRIPTION} LIKE ? AND " +
                "${CalendarContract.Events.DESCRIPTION} LIKE ?"
        val args = arrayOf(
            calendarId.toString(),
            "%$SYNC_TAG%",
            "%$semesterTag%"
        )
        return contentResolver.delete(CalendarContract.Events.CONTENT_URI, selection, args)
    }

    private fun createRecurringEvent(
        calendarId: Long,
        slot: Map<*, *>,
        semesterId: String,
        version: Int,
        startEpochMs: Long,
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

        val startMillis = nextOccurrenceMillisFromBase(day, slot["startTime"]?.toString(), startEpochMs) ?: return false
        var endMillis = nextOccurrenceMillisFromBase(day, slot["endTime"]?.toString(), startMillis) ?: return false
        if (untilEpochMs != null && startMillis > untilEpochMs) return false
        if (endMillis <= startMillis) endMillis = startMillis + 30 * 60 * 1000

        val title = renderTemplate(titleTemplate, slot).ifBlank {
            slot["name"]?.toString()?.ifBlank { "Class" } ?: "Class"
        }
        val location = renderTemplate(locationTemplate, slot).ifBlank {
            buildLocation(slot["block"]?.toString(), slot["roomNo"]?.toString())
        }
        val plainDescription = renderTemplate(descriptionTemplate, slot).ifBlank {
            "Course: ${slot["courseCode"] ?: ""}\nType: ${slot["courseType"] ?: ""}\nSlot: ${slot["slot"] ?: ""}\nFaculty: ${slot["faculty"] ?: ""}"
        }
        val trackedDescription = buildTrackedDescription(plainDescription, semesterId, version)
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
        val uri = contentResolver.insert(CalendarContract.Events.CONTENT_URI, values) ?: return false
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
        }
        return true
    }

    private fun buildTrackedDescription(description: String, semesterId: String, version: Int): String {
        val cleaned = description
            .split("\n")
            .filterNot { it.trim().startsWith(TAG_PREFIX) || it.trim() == SYNC_TAG }
            .joinToString("\n")
            .trim()
        val tags = "${semesterTag(semesterId)}\n${versionTag(version)}\n$SYNC_TAG"
        return if (cleaned.isEmpty()) tags else "$cleaned\n$tags"
    }

    private fun semesterTag(semesterId: String): String = "${TAG_PREFIX}${semesterId.trim()}"
    private fun versionTag(version: Int): String = "${TAG_PREFIX}v$version"

    private fun extractVersion(description: String): Int? {
        val match = Regex("""vitapmate-v(\d+)""", RegexOption.IGNORE_CASE).find(description)
        return match?.groupValues?.getOrNull(1)?.toIntOrNull()
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

        val now = Calendar.getInstance().apply { timeInMillis = baseMillis }
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

    private fun toCalendarDay(dayCode: String): Int? = when (dayCode) {
        "MON" -> Calendar.MONDAY
        "TUE" -> Calendar.TUESDAY
        "WED" -> Calendar.WEDNESDAY
        "THU" -> Calendar.THURSDAY
        "FRI" -> Calendar.FRIDAY
        "SAT" -> Calendar.SATURDAY
        "SUN" -> Calendar.SUNDAY
        else -> null
    }

    private fun toByDay(dayCode: String): String? = when (dayCode) {
        "MON" -> "MO"
        "TUE" -> "TU"
        "WED" -> "WE"
        "THU" -> "TH"
        "FRI" -> "FR"
        "SAT" -> "SA"
        "SUN" -> "SU"
        else -> null
    }

    private fun startOfDay(epochMillis: Long): Long {
        val cal = Calendar.getInstance().apply {
            timeInMillis = epochMillis
            set(Calendar.HOUR_OF_DAY, 0)
            set(Calendar.MINUTE, 0)
            set(Calendar.SECOND, 0)
            set(Calendar.MILLISECOND, 0)
        }
        return cal.timeInMillis
    }

    private fun upsertUntil(rrule: String, epochMillis: Long): String {
        val until = "UNTIL=${toRruleUntil(epochMillis)}"
        val regex = Regex("""(^|;)UNTIL=[^;]+""")
        return if (regex.containsMatchIn(rrule)) {
            rrule.replace(regex, "$1$until")
        } else {
            "$rrule;$until"
        }
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
