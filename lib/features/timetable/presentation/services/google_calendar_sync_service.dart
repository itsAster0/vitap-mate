import 'package:flutter/services.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class CalendarAppInfo {
  final String packageName;
  final String label;

  const CalendarAppInfo({required this.packageName, required this.label});

  factory CalendarAppInfo.fromMap(Map<Object?, Object?> map) {
    return CalendarAppInfo(
      packageName: (map["packageName"] ?? "").toString(),
      label: (map["label"] ?? "").toString(),
    );
  }
}

class CalendarSyncResult {
  final int created;
  final String? calendarName;

  const CalendarSyncResult({required this.created, this.calendarName});
}

class GoogleCalendarSyncService {
  static const MethodChannel _channel = MethodChannel(
    "vitapmate/google_calendar_sync",
  );

  static Future<CalendarSyncResult> syncTimetable(
    TimetableData timetable,
    DateTime untilDate, {
    String? accountName,
    String titleTemplate = "{name}",
    String descriptionTemplate =
        "Course: {courseCode}\nType: {courseType}\nSlot: {slot}\nFaculty: {faculty}\nSEM:{semesterId}",
    String locationTemplate = "{block}-{roomNo}",
  }) async {
    final slots =
        timetable.slots
            .where((slot) => slot.serial != "-1")
            .map(
              (slot) => {
                "serial": slot.serial,
                "day": slot.day,
                "slot": slot.slot,
                "courseCode": slot.courseCode,
                "courseType": slot.courseType,
                "roomNo": slot.roomNo,
                "block": slot.block,
                "startTime": slot.startTime,
                "endTime": slot.endTime,
                "name": slot.name,
                "faculty": slot.faculty,
              },
            )
            .toList();

    final raw = await _channel
        .invokeMethod<dynamic>("syncTimetableToGoogleCalendar", {
          "semesterId": timetable.semesterId,
          "untilEpochMs": untilDate.millisecondsSinceEpoch,
          "accountName": accountName,
          "titleTemplate": titleTemplate,
          "descriptionTemplate": descriptionTemplate,
          "locationTemplate": locationTemplate,
          "slots": slots,
        });

    if (raw is int) {
      return CalendarSyncResult(created: raw);
    }
    if (raw is Map<Object?, Object?>) {
      return CalendarSyncResult(
        created: (raw["created"] as num?)?.toInt() ?? 0,
        calendarName: raw["calendarName"]?.toString(),
      );
    }
    return const CalendarSyncResult(created: 0);
  }

  static Future<List<CalendarAppInfo>> getAvailableCalendarApps() async {
    final raw = await _channel.invokeMethod<List<dynamic>>(
      "getAvailableCalendarApps",
    );
    if (raw == null) return const [];
    return raw
        .whereType<Map<Object?, Object?>>()
        .map(CalendarAppInfo.fromMap)
        .toList();
  }

  static Future<List<String>> getGoogleAccounts() async {
    final raw = await _channel.invokeMethod<List<dynamic>>("getGoogleAccounts");
    if (raw == null) return const [];
    return raw
        .map((e) => e.toString())
        .where((e) => e.trim().isNotEmpty)
        .toList();
  }

  static Future<bool> openCalendarApp(String packageName) async {
    final isOpened = await _channel.invokeMethod<bool>("openCalendarApp", {
      "packageName": packageName,
    });
    return isOpened ?? false;
  }
}
