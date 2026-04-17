import 'dart:io';

import 'package:flutter/services.dart';
import 'package:vitapmate/features/timetable/presentation/utils/timetable_slot_merge.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class CalendarInfo {
  final String id;
  final String name;
  final String accountName;

  const CalendarInfo({
    required this.id,
    required this.name,
    required this.accountName,
  });

  factory CalendarInfo.fromMap(Map<Object?, Object?> map) {
    return CalendarInfo(
      id: (map["id"] ?? "").toString(),
      name: (map["name"] ?? "").toString(),
      accountName: (map["accountName"] ?? "").toString(),
    );
  }
}

class CalendarSyncResult {
  final int created;
  final String? calendarName;
  final int version;

  const CalendarSyncResult({
    required this.created,
    required this.version,
    this.calendarName,
  });
}

class CalendarSyncService {
  static const MethodChannel _channel = MethodChannel(
    "vitapmate/google_calendar_sync",
  );

  static Future<CalendarSyncResult> syncTimetable(
    TimetableData timetable,
    DateTime startDate,
    DateTime endDate, {
    required String calendarId,
    int reminderMinutes = 10,
    String titleTemplate = "{name}",
    String descriptionTemplate =
        "Course: {courseCode}\nType: {courseType}\nSlot: {slot}\nFaculty: {faculty}",
    String locationTemplate = "{block}-{roomNo}",
  }) async {
    if (!Platform.isAndroid) {
      throw Exception("Calendar sync is currently Android-only.");
    }
    final rawSlots = timetable.slots
        .where((slot) => slot.serial != "-1")
        .toList();
    final preparedSlots = _mergeLabsByDay(rawSlots);
    final slots = preparedSlots
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

    final raw = await _channel.invokeMethod<dynamic>("syncTimetableRecurring", {
      "calendarId": calendarId,
      "semesterId": timetable.semesterId,
      "startEpochMs": startDate.millisecondsSinceEpoch,
      "untilEpochMs": endDate.millisecondsSinceEpoch,
      "reminderMinutes": reminderMinutes,
      "titleTemplate": titleTemplate,
      "descriptionTemplate": descriptionTemplate,
      "locationTemplate": locationTemplate,
      "slots": slots,
    });

    if (raw is Map<Object?, Object?>) {
      return CalendarSyncResult(
        created: (raw["created"] as num?)?.toInt() ?? 0,
        version: (raw["version"] as num?)?.toInt() ?? 1,
        calendarName: raw["calendarName"]?.toString(),
      );
    }
    return const CalendarSyncResult(created: 0, version: 1);
  }

  static Future<(int deleted, String? calendarName)> clearAllSyncedEvents({
    required String calendarId,
    required String semesterId,
  }) async {
    if (!Platform.isAndroid) return (0, null);

    final raw = await _channel.invokeMethod<dynamic>("clearAllSyncedEvents", {
      "calendarId": calendarId,
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

  static Future<List<CalendarInfo>> getWritableCalendars() async {
    if (!Platform.isAndroid) return const [];
    final raw = await _channel.invokeMethod<List<dynamic>>(
      "getWritableCalendars",
    );
    if (raw == null) return const [];
    return raw
        .whereType<Map<Object?, Object?>>()
        .map(CalendarInfo.fromMap)
        .where((c) => c.id.trim().isNotEmpty)
        .toList();
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
