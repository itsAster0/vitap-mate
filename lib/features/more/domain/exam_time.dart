import 'package:intl/intl.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// Exam day from VTOP's "17-Aug-2026", or null when not announced.
DateTime? examDayOf(ExamScheduleRecord exam) {
  final raw = exam.examDate.trim();
  if (raw.isEmpty || raw == '-') return null;
  for (final pattern in ['dd-MMM-yyyy', 'd-MMM-yyyy', 'dd-MM-yyyy']) {
    try {
      return DateFormat(pattern, 'en_US').parseLoose(raw);
    } catch (_) {}
  }
  return null;
}

/// Start of the exam, from "02:00 PM - 03:30 PM" on its day. Falls back to
/// the end of the day when the time is missing.
DateTime? examStartOf(ExamScheduleRecord exam) {
  final day = examDayOf(exam);
  if (day == null) return null;
  final start = parseClock(exam.examTime.split('-').first);
  if (start == null) return DateTime(day.year, day.month, day.day, 23, 59);
  return DateTime(day.year, day.month, day.day, start.$1, start.$2);
}

/// "02:00 PM" → (14, 0).
(int, int)? parseClock(String raw) {
  final match = RegExp(
    r'^\s*(\d{1,2})(?::(\d{2}))?\s*([AaPp][Mm])?\s*$',
  ).firstMatch(raw);
  if (match == null) return null;
  var hour = int.tryParse(match.group(1) ?? '');
  final minute = int.tryParse(match.group(2) ?? '') ?? 0;
  final period = match.group(3)?.toUpperCase();
  if (hour == null || hour > 23 || minute > 59) return null;
  if (period == 'AM' && hour == 12) hour = 0;
  if (period == 'PM' && hour != 12) hour += 12;
  return (hour, minute);
}

/// True when a VTOP field holds a real value rather than "-" or blank.
bool hasValue(String raw) => raw.trim().isNotEmpty && raw.trim() != '-';
