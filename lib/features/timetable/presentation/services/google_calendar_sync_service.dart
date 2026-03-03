import 'package:flutter/services.dart';
import 'package:vitapmate/features/timetable/presentation/utils/timetable_slot_merge.dart';
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
    DateTime startDate,
    DateTime endDate, {
    String? accountName,
    int reminderMinutes = 10,
    String titleTemplate = "{name}",
    String descriptionTemplate =
        "Course: {courseCode}\nType: {courseType}\nSlot: {slot}\nFaculty: {faculty}",
    String locationTemplate = "{block}-{roomNo}",
  }) async {
    final rawSlots =
        timetable.slots.where((slot) => slot.serial != "-1").toList();
    final preparedSlots = _mergeLabsByDay(rawSlots);
    final slots =
        preparedSlots
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
          "startEpochMs": startDate.millisecondsSinceEpoch,
          "untilEpochMs": endDate.millisecondsSinceEpoch,
          "accountName": accountName,
          "reminderMinutes": reminderMinutes,
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

  static Future<(int deleted, String? calendarName)> clearAllSyncedEvents({
    String? accountName,
    required String semesterId,
  }) async {
    final raw = await _channel.invokeMethod<dynamic>("clearAllSyncedEvents", {
      "accountName": accountName,
      "semesterId": semesterId,
    });
    if (raw is Map<Object?, Object?>) {
      return (
        (raw["deleted"] as num?)?.toInt() ?? 0,
        raw["calendarName"]?.toString(),
      );
    }
    return (0, null);
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

  static List<TimetableSlot> _mergeLabsByDay(List<TimetableSlot> slots) {
    final grouped = <String, List<TimetableSlot>>{};
    for (final slot in slots) {
      grouped.putIfAbsent(slot.day, () => []).add(slot);
    }

    final out = <TimetableSlot>[];
    for (final daySlots in grouped.values) {
      out.addAll(mergeLabsSloths(daySlots));
    }
    return out;
  }
}
