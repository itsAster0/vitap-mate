import 'dart:convert';

import 'package:flutter_local_notifications/flutter_local_notifications.dart';
import 'package:meta/meta.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/features/background/change_detection/change_detector.dart';

enum ChangeAlertType { attendance, marks, timetable, examSchedule }

class ChangeAlertNotificationService {
  static final FlutterLocalNotificationsPlugin _notifications =
      FlutterLocalNotificationsPlugin();

  static const String channelId = 'change_alerts_v1';
  static const String _channelName = 'VTOP Changes';
  static const String _payloadType = 'change_alert';
  static const String _masterKey = 'settings_change_alerts_enabled';
  static const String _attendanceKey = 'settings_change_alerts_attendance';
  static const String _marksKey = 'settings_change_alerts_marks';
  static const String _timetableKey = 'settings_change_alerts_timetable';
  static const String _examScheduleKey =
      'settings_change_alerts_exam_schedule';

  static bool _initialized = false;

  static Future<void> ensureInitialized() async {
    if (_initialized) return;
    const androidSettings = AndroidInitializationSettings(
      '@mipmap/launcher_icon',
    );
    await _notifications.initialize(
      settings: const InitializationSettings(android: androidSettings),
    );
    await _notifications
        .resolvePlatformSpecificImplementation<
          AndroidFlutterLocalNotificationsPlugin
        >()
        ?.createNotificationChannel(
          const AndroidNotificationChannel(
            channelId,
            _channelName,
            description: 'Alerts when your VTOP data changes',
            importance: Importance.high,
          ),
        );
    _initialized = true;
  }

  static Future<bool> isEnabled(ChangeAlertType type) async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.reload();
    if (!(prefs.getBool(_masterKey) ?? true)) return false;
    switch (type) {
      case ChangeAlertType.attendance:
        return prefs.getBool(_attendanceKey) ?? true;
      case ChangeAlertType.marks:
        return prefs.getBool(_marksKey) ?? true;
      case ChangeAlertType.timetable:
        return prefs.getBool(_timetableKey) ?? true;
      case ChangeAlertType.examSchedule:
        return prefs.getBool(_examScheduleKey) ?? true;
    }
  }

  static Future<void> showDataChange({
    required ChangeAlertType type,
    required String semesterId,
    required DataChangeSummary summary,
  }) async {
    await ensureInitialized();
    if (!await isEnabled(type)) return;

    final prefs = await SharedPreferences.getInstance();
    final hashKey = 'alerts_last_hash_${type.name}_$semesterId';
    final hash = _stableHash('${summary.title}|${summary.body}');
    if (prefs.getInt(hashKey) == hash) return;
    await prefs.setInt(hashKey, hash);

    final id =
        _stableHash('$channelId|${type.name}|$semesterId') & 0x7fffffff;

    await _notifications.show(
      id: id,
      title: summary.title,
      body: summary.body,
      notificationDetails: NotificationDetails(
        android: AndroidNotificationDetails(
          channelId,
          _channelName,
          channelDescription: 'Alerts when your VTOP data changes',
          importance: Importance.high,
          priority: Priority.high,
          styleInformation: BigTextStyleInformation(summary.body),
        ),
      ),
      payload: jsonEncode({
        'type': _payloadType,
        'dataType': type.name,
        'semesterId': semesterId,
      }),
    );
  }

  @visibleForTesting
  static int stableHashForTest(String input) => _stableHash(input);

  static int _stableHash(String input) {
    var hash = 0xcbf29ce484222325;
    for (final codeUnit in input.codeUnits) {
      hash ^= codeUnit;
      hash = (hash * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF;
    }
    return hash;
  }
}
