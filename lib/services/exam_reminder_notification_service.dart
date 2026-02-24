import 'dart:convert';

import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:intl/intl.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:timezone/data/latest.dart' as tz;
import 'package:timezone/timezone.dart' as tz;
import 'package:vitapmate/src/api/vtop/types.dart';

class ExamReminderNotificationService {
  static final FlutterLocalNotificationsPlugin _notifications =
      FlutterLocalNotificationsPlugin();
  static const String channelId = "exam_reminders_v1";
  static const String _payloadType = "exam_reminder";
  static const String _enabledKey = "settings_exam_notifications_enabled";
  static const String _beforeKey = "settings_exam_notify_before_minutes";
  static bool _tzInitialized = false;

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
      const InitializationSettings(android: androidSettings),
    );

    await _notifications
        .resolvePlatformSpecificImplementation<
          AndroidFlutterLocalNotificationsPlugin
        >()
        ?.createNotificationChannel(
          const AndroidNotificationChannel(
            channelId,
            "Exam reminders",
            description: "Notifications before upcoming exams",
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
          await _notifications.cancel(request.id);
        }
      } catch (_) {
        continue;
      }
    }
  }

  static Future<void> syncFromExamSchedule(ExamScheduleData data) async {
    await ensureInitialized();
    final prefs = await SharedPreferences.getInstance();
    await prefs.reload();
    final enabled = prefs.getBool(_enabledKey) ?? false;
    if (!enabled) {
      await cancelAll();
      return;
    }
    final beforeMinutes = prefs.getInt(_beforeKey) ?? 10;
    await cancelAll();

    var scheduled = 0;

    for (final group in data.exams) {
      for (final exam in group.records) {
        final examDateTime = _parseExamDateTime(exam);
        if (examDateTime == null) {
          continue;
        }
        final remindAt = examDateTime.subtract(
          Duration(minutes: beforeMinutes),
        );
        if (!remindAt.isAfter(DateTime.now())) {
          continue;
        }

        final id =
            Object.hash(
              data.semesterId,
              exam.courseId,
              exam.examDate,
              exam.examTime,
            ) &
            0x7fffffff;

        final payload = jsonEncode({
          "type": _payloadType,
          "semesterId": data.semesterId,
          "courseId": exam.courseId,
          "courseName": exam.courseName,
        });

        await _notifications.zonedSchedule(
          id,
          "Exam Reminder",
          "${exam.courseCode} ${exam.courseName} in $beforeMinutes minutes",
          tz.TZDateTime.from(remindAt, tz.local),
          NotificationDetails(
            android: AndroidNotificationDetails(
              channelId,
              "Exam reminders",
              channelDescription: "Notifications before upcoming exams",
              importance: Importance.high,
              priority: Priority.high,
            ),
          ),
          payload: payload,
          androidScheduleMode: AndroidScheduleMode.inexactAllowWhileIdle,
        );
        scheduled++;
      }
    }
  }

  static DateTime? _parseExamDateTime(ExamScheduleRecord exam) {
    final date = DateTime.tryParse(exam.examDate);
    if (date == null) return null;

    if (date.hour != 0 || date.minute != 0 || date.second != 0) return date;
    final parsedTime =
        _tryParseTime(exam.reportingTime) ?? _tryParseTime(exam.examTime);
    if (parsedTime == null) {
      return DateTime(date.year, date.month, date.day, 9, 0);
    }
    return DateTime(
      date.year,
      date.month,
      date.day,
      parsedTime.hour,
      parsedTime.minute,
    );
  }

  static DateTime? _tryParseTime(String time) {
    final raw = time.trim();
    if (raw.isEmpty) return null;
    final normalized = raw.toUpperCase().replaceAll(".", "");
    final formats = <String>[
      "HH:mm",
      "HH:mm:ss",
      "H:mm",
      "h:mm a",
      "hh:mm a",
      "h a",
      "hh a",
    ];
    for (final format in formats) {
      try {
        return DateFormat(format).parseStrict(normalized);
      } catch (_) {
        continue;
      }
    }
    return null;
  }
}
