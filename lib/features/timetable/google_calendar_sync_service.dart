import 'dart:io';

import 'package:flutter/services.dart';
import 'package:vitapmate/features/timetable/logic/timetable_slot_merge.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

typedef CalendarInfo = ({String id, String name, String accountName});

CalendarInfo _calendarInfo(Map<Object?, Object?> map) => (
  id: (map['id'] ?? '').toString(),
  name: (map['name'] ?? '').toString(),
  accountName: (map['accountName'] ?? '').toString(),
);

typedef CalendarSyncResult = ({int created, int version, String? calendarName});

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
    // Labs are merged per day, in the order days first appear.
    final byDay = <String, List<TimetableSlot>>{};
    for (final slot in timetable.slots) {
      if (slot.serial != '-1') byDay.putIfAbsent(slot.day, () => []).add(slot);
    }
    final slots = [
      for (final day in byDay.values)
        for (final slot in mergeLabsSloths(day))
          {
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
    ];

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

    return raw is Map<Object?, Object?>
        ? (
            created: (raw["created"] as num?)?.toInt() ?? 0,
            version: (raw["version"] as num?)?.toInt() ?? 1,
            calendarName: raw["calendarName"]?.toString(),
          )
        : (created: 0, version: 1, calendarName: null);
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
    return raw is Map<Object?, Object?>
        ? (
            (raw["deleted"] as num?)?.toInt() ?? 0,
            raw["calendarName"]?.toString(),
          )
        : (0, null);
  }

  static Future<List<CalendarInfo>> getWritableCalendars() async {
    if (!Platform.isAndroid) return const [];
    final raw = await _channel.invokeMethod<List<dynamic>>(
      "getWritableCalendars",
    );
    return [
      for (final map in [...?raw?.whereType<Map<Object?, Object?>>()])
        if (_calendarInfo(map) case final c when c.id.trim().isNotEmpty) c,
    ];
  }
}
