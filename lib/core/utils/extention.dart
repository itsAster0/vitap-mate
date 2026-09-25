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

extension PersonName on String {
  /// "PRASHANTH KUMAR" → "Prashanth Kumar". VTOP sends some names in capitals;
  /// mixed-case names are left alone so initials like "Md" survive.
  String get asPersonName {
    final name = trim();
    if (name != name.toUpperCase()) return name;
    return name
        .split(RegExp(r'\s+'))
        .map(
          (w) => w.length <= 1 || w.contains('.')
              ? w
              : '${w[0]}${w.substring(1).toLowerCase()}',
        )
        .join(' ');
  }
}
