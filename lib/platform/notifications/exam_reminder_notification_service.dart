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

    await _notifications.initialize(
      settings: const InitializationSettings(
        android: AndroidInitializationSettings('@mipmap/launcher_icon'),
      ),
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
    for (final request in await _notifications.pendingNotificationRequests()) {
      try {
        if (jsonDecode(request.payload ?? 'null') case <String, dynamic>{
          'type': _payloadType,
        }) {
          await _notifications.cancel(id: request.id);
        }
      } catch (_) {}
    }
  }

  static Future<void> syncFromExamSchedule(ExamScheduleData data) async {
    await ensureInitialized();
    final prefs = await SharedPreferences.getInstance();
    await prefs.reload();
    await cancelAll();
    if (!(prefs.getBool(_enabledKey) ?? false)) return;
    final beforeMinutes = prefs.getInt(_beforeKey) ?? 10;

    for (final group in data.exams) {
      for (final exam in group.records) {
        final examDateTime = _parseExamDateTime(exam);
        if (examDateTime == null) continue;
        final remindAt = examDateTime.subtract(
          Duration(minutes: beforeMinutes),
        );
        if (!remindAt.isAfter(DateTime.now())) continue;

        final id =
            Object.hash(
              data.semesterId,
              exam.courseId,
              exam.examDate,
              exam.examTime,
            ) &
            0x7fffffff;

        await _notifications.zonedSchedule(
          id: id,
          title: "Exam Reminder",
          body:
              "${exam.courseCode} ${exam.courseName} in $beforeMinutes minutes",
          scheduledDate: tz.TZDateTime.from(remindAt, tz.local),
          notificationDetails: const NotificationDetails(
            android: AndroidNotificationDetails(
              channelId,
              "Exam reminders",
              channelDescription: "Notifications before upcoming exams",
              importance: Importance.high,
              priority: Priority.high,
            ),
          ),
          payload: jsonEncode({
            "type": _payloadType,
            "semesterId": data.semesterId,
            "courseId": exam.courseId,
            "courseName": exam.courseName,
          }),
          androidScheduleMode: AndroidScheduleMode.inexactAllowWhileIdle,
        );
      }
    }
  }

  static DateTime? _parseExamDateTime(ExamScheduleRecord exam) {
    final date = DateTime.tryParse(exam.examDate);
    if (date == null) return null;
    if (date.hour != 0 || date.minute != 0 || date.second != 0) return date;
    final time =
        _tryParseTime(exam.reportingTime) ?? _tryParseTime(exam.examTime);
    return DateTime(
      date.year,
      date.month,
      date.day,
      time?.hour ?? 9,
      time?.minute ?? 0,
    );
  }

  static const _timeFormats = [
    "HH:mm",
    "HH:mm:ss",
    "H:mm",
    "h:mm a",
    "hh:mm a",
    "h a",
    "hh a",
  ];

  static DateTime? _tryParseTime(String time) {
    final raw = time.trim();
    if (raw.isEmpty) return null;
    final normalized = raw.toUpperCase().replaceAll(".", "");
    for (final format in _timeFormats) {
      try {
        return DateFormat(format).parseStrict(normalized);
      } catch (_) {}
    }
    return null;
  }
}
