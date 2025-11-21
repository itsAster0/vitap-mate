import 'package:vitapmate/src/api/vtop/types.dart';

extension IslabAttendanceRecord on AttendanceRecord {
  bool islab() => courseType.endsWith("LA") || courseType.endsWith("LO");
}

extension IslabMarksRecord on MarksRecord {
  bool islab() =>
      coursetype.toLowerCase().endsWith("lab") ||
      coursetype.toLowerCase().endsWith("lab only");
}

extension IslabTimetable on TimetableSlot {
  bool islab() => courseType.endsWith("LA") || courseType.endsWith("LO");
}
