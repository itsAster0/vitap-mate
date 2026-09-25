import 'package:flutter/widgets.dart';
import 'package:vitapmate/core/utils/extention.dart';

/// "14:05" → "2:05 PM", or unchanged when the device uses 24-hour time.
String to12H(String time, BuildContext context) {
  if (MediaQuery.of(context).alwaysUse24HourFormat) return time;
  final parts = time.split(':');
  final hour24 = int.parse(parts[0]);
  final hour12 = hour24 % 12 == 0 ? 12 : hour24 % 12;
  return '$hour12:${parts[1]} ${hour24 >= 12 ? 'PM' : 'AM'}';
}

/// Any VTOP clock ("02:00 PM", "14:00", "9:5") in the app's one style:
/// "2:00 PM", or "14:00" when the device uses 24-hour time. Unparseable
/// input comes back trimmed.
String formatClock(String raw, BuildContext context) {
  final match = RegExp(
    r'^\s*(\d{1,2}):(\d{2})\s*([AaPp][Mm])?\s*$',
  ).firstMatch(raw);
  if (match == null) return raw.trim();
  var hour = int.parse(match.group(1)!);
  final period = match.group(3)?.toUpperCase();
  if (period == 'AM' && hour == 12) hour = 0;
  if (period == 'PM' && hour != 12) hour += 12;
  return to12H('${hour.toString().padLeft(2, '0')}:${match.group(2)}', context);
}

/// Minutes since midnight for an "HH:mm" string.
int minutesOf(String time) {
  final parts = time.split(':');
  return int.parse(parts[0]) * 60 + int.parse(parts[1]);
}

/// "2 hr 30 min", "45 min", "1 hr".
String formatMinutes(int minutes) {
  final hours = minutes ~/ 60;
  final rest = minutes.remainder(60);
  return [
    if (hours > 0) '$hours hr',
    if (rest > 0 || hours == 0) '$rest min',
  ].join(' ');
}

/// Faculty strings arrive as "Name - SCHOOL"; keep the name.
String facultyName(String value) {
  final index = value.lastIndexOf('-');
  return (index > 0 ? value.substring(0, index) : value).asPersonName;
}

/// Monday-to-Sunday dates of the week shown in the weekly grid. On Sundays it
/// shows the coming week, since the current one is over.
List<DateTime> getCurrentWeekDates() {
  final now = DateTime.now();
  final referenceDate = now.weekday == DateTime.sunday
      ? now.add(const Duration(days: 1))
      : now;
  final startOfWeek = referenceDate.subtract(
    Duration(days: referenceDate.weekday - 1),
  );
  return List.generate(7, (index) => startOfWeek.add(Duration(days: index)));
}
