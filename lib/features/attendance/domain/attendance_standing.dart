import 'package:vitapmate/core/utils/extention.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

/// Minimum attendance VIT requires.
const attendanceThreshold = 75.0;

/// Where a course stands against [attendanceThreshold], using the same
/// strict-75% arithmetic as the attendance calculator.
class AttendanceStanding {
  const AttendanceStanding({
    required this.attended,
    required this.total,
    this.reported,
  });

  factory AttendanceStanding.of(AttendanceRecord record) => AttendanceStanding(
    attended: int.tryParse(record.classesAttended.trim()) ?? 0,
    total: int.tryParse(record.totalClasses.trim()) ?? 0,
    reported: double.tryParse(
      record.attendancePercentage.replaceAll('%', '').trim(),
    ),
  );

  final int attended;
  final int total;

  /// The percentage VTOP shows. VTOP rounds up, so this can read higher than
  /// [percent]; display it so numbers match the portal.
  final double? reported;

  double get displayPercent => reported ?? percent;

  double get percent => total == 0 ? 0 : attended / total * 100;

  bool get isSafe => total == 0 || percent >= attendanceThreshold;

  /// Classes that can be missed in a row while staying at or above 75%.
  int get canSkip {
    if (total == 0) return 0;
    final k = ((4 * attended - 3 * total) / 3).floor();
    return k > 0 ? k : 0;
  }

  /// Consecutive classes needed to climb back to 75%.
  int get mustAttend {
    final n = 3 * total - 4 * attended;
    return n > 0 ? n : 0;
  }

  /// One-line guidance, e.g. "Can skip 3" or "Attend 2".
  String get advice {
    if (total == 0) return 'No classes yet';
    if (!isSafe) return 'Attend $mustAttend';
    if (canSkip == 0) return "Don't skip";
    return 'Can skip $canSkip';
  }
}

/// The course code ("CSE4007") from the "CODE - Name - Type" course name.
String courseCodeOf(AttendanceRecord record) =>
    record.courseName.split(' - ').first.trim();

/// Finds the attendance record for a timetable class (same course, and lab
/// matched to lab).
AttendanceRecord? attendanceForSlot(
  Iterable<AttendanceRecord> records,
  TimetableSlot slot,
) {
  final wantLab = slot.kind == ClassKind.lab;
  for (final record in records) {
    if (courseCodeOf(record) == slot.courseCode && record.islab() == wantLab) {
      return record;
    }
  }
  return null;
}
