import 'dart:convert';

import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:timezone/data/latest.dart' as tz;
import 'package:timezone/timezone.dart' as tz;
import 'package:vitapmate/features/calendar/domain/semester_calendar.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class ClassReminderNotificationService {
  static final FlutterLocalNotificationsPlugin _notifications =
      FlutterLocalNotificationsPlugin();

  static const String channelId = "class_reminders_v1";
  static const String _pauseActionId = "pause_class_for_today";
  static const String _payloadType = "class_reminder";
  static const String _enabledKey = "settings_class_notifications_enabled";
  static const String _beforeKey = "settings_class_notify_before_minutes";
  static const String _pauseUntilKey = "settings_class_pause_until_millis";
  static bool _tzInitialized = false;

  /// How far ahead dated reminders are scheduled. Each timetable or
  /// calendar load, and each background sync, tops them up.
  static const _datedHorizonDays = 14;

  static Future<void> ensureInitialized() async {
    if (!_tzInitialized) {
      tz.initializeTimeZones();
      try {
        tz.setLocalLocation(tz.getLocation("Asia/Kolkata"));
      } catch (_) {}
      _tzInitialized = true;
    }

    const androidSettings = AndroidInitializationSettings(
      '@mipmap/launcher_icon',
    );
    await _notifications.initialize(
      settings: const InitializationSettings(android: androidSettings),
      onDidReceiveNotificationResponse: (response) {
        handleNotificationResponse(response);
      },
      onDidReceiveBackgroundNotificationResponse:
          classReminderBackgroundTapHandler,
    );

    await _notifications
        .resolvePlatformSpecificImplementation<
          AndroidFlutterLocalNotificationsPlugin
        >()
        ?.createNotificationChannel(
          const AndroidNotificationChannel(
            channelId,
            "Class reminders",
            description: "Notifications before your classes",
            importance: Importance.high,
          ),
        );
  }

  static Future<void> cancelAll() async {
    final pending = await _notifications.pendingNotificationRequests();
    for (final request in pending) {
      try {
        final payload = request.payload;
        if (payload == null) continue;
        final parsed = jsonDecode(payload);
        if (parsed is Map<String, dynamic> && parsed["type"] == _payloadType) {
          await _notifications.cancel(id: request.id);
        }
      } catch (_) {
        continue;
      }
    }
  }

  static Future<void> pauseForToday() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.reload();
    final now = DateTime.now();
    final until = DateTime(now.year, now.month, now.day, 23, 59, 59);
    await prefs.setInt(_pauseUntilKey, until.millisecondsSinceEpoch);
    await cancelAll();
  }

  static Future<bool> handleNotificationResponse(
    NotificationResponse response,
  ) async {
    if (response.actionId == _pauseActionId) {
      await pauseForToday();
      return true;
    }
    final payload = response.payload;
    if (payload == null || payload.isEmpty) return false;
    try {
      final parsed = jsonDecode(payload);
      if (parsed is Map<String, dynamic> && parsed["type"] == _payloadType) {
        return true;
      }
    } catch (_) {
      return false;
    }
    return false;
  }

  /// Schedules a reminder before every class. With a [calendar], reminders
  /// are set for the actual class days of the next two weeks, so holidays,
  /// exam days and labs after the LAB FAT get none; without one they repeat
  /// weekly.
  static Future<void> syncFromTimetable(
    TimetableData data, {
    SemesterCalendar? calendar,
  }) async {
    await ensureInitialized();
    final prefs = await SharedPreferences.getInstance();
    await prefs.reload();
    final enabled = prefs.getBool(_enabledKey) ?? false;
    final pauseUntilMillis = prefs.getInt(_pauseUntilKey);
    final paused =
        pauseUntilMillis != null &&
        DateTime.now().isBefore(
          DateTime.fromMillisecondsSinceEpoch(pauseUntilMillis),
        );

    if (!enabled || paused) {
      await cancelAll();
      return;
    }

    final beforeMinutes = prefs.getInt(_beforeKey) ?? 10;
    await cancelAll();

    if (calendar != null) {
      await _scheduleDated(data, calendar, beforeMinutes);
      return;
    }

    for (final slot in data.slots) {
      if (slot.serial == "-1") {
        continue;
      }
      final weekday = _weekdayFromSlot(slot.day);
      final classTime = _parseTime(slot.startTime);
      if (weekday == null || classTime == null) {
        continue;
      }

      var nextClass = _nextDateForWeekday(weekday, classTime);
      var remindAt = nextClass.subtract(Duration(minutes: beforeMinutes));
      if (!remindAt.isAfter(DateTime.now())) {
        nextClass = nextClass.add(const Duration(days: 7));
        remindAt = nextClass.subtract(Duration(minutes: beforeMinutes));
      }

      final id =
          Object.hash(
            data.semesterId,
            slot.day,
            slot.startTime,
            slot.courseCode,
            slot.slot,
          ) &
          0x7fffffff;

      final payload = jsonEncode({
        "type": _payloadType,
        "semesterId": data.semesterId,
        "courseCode": slot.courseCode,
        "slot": slot.slot,
      });

      await _notifications.zonedSchedule(
        id: id,
        title: "Class Reminder",
        body: "${slot.courseCode} in $beforeMinutes min (${slot.startTime})",
        scheduledDate: tz.TZDateTime.from(remindAt, tz.local),
        notificationDetails: _details,
        payload: payload,
        androidScheduleMode: AndroidScheduleMode.inexactAllowWhileIdle,
        matchDateTimeComponents: DateTimeComponents.dayOfWeekAndTime,
      );
    }
  }

  static const _details = NotificationDetails(
    android: AndroidNotificationDetails(
      channelId,
      "Class reminders",
      channelDescription: "Notifications before your classes",
      importance: Importance.high,
      priority: Priority.high,
      actions: [
        AndroidNotificationAction(
          _pauseActionId,
          "Pause for today",
          cancelNotification: true,
        ),
      ],
    ),
  );

  /// One-off reminders for each class the calendar holds in the next
  /// [_datedHorizonDays] days.
  static Future<void> _scheduleDated(
    TimetableData data,
    SemesterCalendar calendar,
    int beforeMinutes,
  ) async {
    final now = DateTime.now();
    for (var offset = 0; offset < _datedHorizonDays; offset++) {
      final day = DateTime(now.year, now.month, now.day + offset);
      for (final slot in data.slots) {
        if (slot.serial == "-1") continue;
        if (_weekdayFromSlot(slot.day) != day.weekday) continue;
        if (!calendar.holdsClasses(day, lab: slot.kind == ClassKind.lab)) {
          continue;
        }
        final classTime = _parseTime(slot.startTime);
        if (classTime == null) continue;
        final remindAt = DateTime(
          day.year,
          day.month,
          day.day,
          classTime.$1,
          classTime.$2,
        ).subtract(Duration(minutes: beforeMinutes));
        if (!remindAt.isAfter(now)) continue;

        final id =
            Object.hash(
              data.semesterId,
              day.millisecondsSinceEpoch,
              slot.startTime,
              slot.courseCode,
              slot.slot,
            ) &
            0x7fffffff;
        await _notifications.zonedSchedule(
          id: id,
          title: "Class Reminder",
          body: "${slot.courseCode} in $beforeMinutes min (${slot.startTime})",
          scheduledDate: tz.TZDateTime.from(remindAt, tz.local),
          notificationDetails: _details,
          payload: jsonEncode({
            "type": _payloadType,
            "semesterId": data.semesterId,
            "courseCode": slot.courseCode,
            "slot": slot.slot,
          }),
          androidScheduleMode: AndroidScheduleMode.inexactAllowWhileIdle,
        );
      }
    }
  }

  static DateTime _nextDateForWeekday(
    int targetWeekday,
    (int hour, int minute) classTime,
  ) {
    final now = DateTime.now();
    final todayAtClass = DateTime(
      now.year,
      now.month,
      now.day,
      classTime.$1,
      classTime.$2,
    );
    final diff = (targetWeekday - now.weekday) % 7;
    final date = now.add(Duration(days: diff));
    final candidate = DateTime(
      date.year,
      date.month,
      date.day,
      classTime.$1,
      classTime.$2,
    );

    if (diff == 0 && !todayAtClass.isAfter(now)) {
      return candidate.add(const Duration(days: 7));
    }
    return candidate;
  }

  static int? _weekdayFromSlot(String day) {
    const map = {
      "MON": DateTime.monday,
      "TUE": DateTime.tuesday,
      "WED": DateTime.wednesday,
      "THU": DateTime.thursday,
      "FRI": DateTime.friday,
      "SAT": DateTime.saturday,
      "SUN": DateTime.sunday,
    };
    return map[day.trim().toUpperCase()];
  }

  static (int, int)? _parseTime(String time) {
    final parts = time.split(":");
    if (parts.length < 2) return null;
    final hour = int.tryParse(parts[0]);
    final minute = int.tryParse(parts[1]);
    if (hour == null || minute == null) return null;
    return (hour, minute);
  }
}

@pragma("vm:entry-point")
Future<void> classReminderBackgroundTapHandler(
  NotificationResponse response,
) async {
  await ClassReminderNotificationService.handleNotificationResponse(response);
}
