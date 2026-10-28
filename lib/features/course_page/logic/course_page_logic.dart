/// Grouping, naming and matching for the course page.
library;

import 'package:vitapmate/src/api/vtop/types.dart';

/// One course code with its parts (theory, lab, project...).
class CourseGroup {
  const CourseGroup({
    required this.code,
    required this.title,
    required this.parts,
  });

  final String code;
  final String title;
  final List<CoursePageCourse> parts;
}

/// Courses grouped by code, in the order VTOP lists them.
List<CourseGroup> groupCourses(List<CoursePageCourse> courses) {
  final groups = <String, List<CoursePageCourse>>{};
  for (final course in courses) {
    groups.putIfAbsent(course.code, () => []).add(course);
  }
  return [
    for (final MapEntry(key: code, value: parts) in groups.entries)
      CourseGroup(code: code, title: parts.first.title, parts: parts),
  ];
}

/// `ETH` → "Theory", `ELA` → "Lab"...
String courseTypeLabel(String type) => switch (type.toUpperCase()) {
  'TH' || 'ETH' => 'Theory',
  'LO' || 'ELA' => 'Lab',
  'EPJ' || 'PJT' => 'Project',
  'SS' => 'Soft skills',
  _ => type,
};

bool isLabType(String type) =>
    const {'LO', 'ELA'}.contains(type.toUpperCase()) ||
    type.toLowerCase().contains('lab');

String _norm(String value) =>
    value.toLowerCase().replaceAll(RegExp(r'\s+'), ' ').trim();

/// Slot tokens: "A1/TA1", "A2+TA2" → {a1, ta1}.
Set<String> _slots(String value) => {
  for (final token in value.toLowerCase().split(RegExp(r'[^a-z0-9]+')))
    if (token.isNotEmpty) token,
};

/// The class the student attends, judged from the timetable: same course
/// code and faculty, and when the faculty teaches several sections, the
/// one sharing a slot. Null when the timetable does not settle it.
CoursePageClass? myClass(
  List<CoursePageClass> classes,
  TimetableData? timetable,
) {
  if (timetable == null || classes.isEmpty) return null;
  final mine = timetable.slots
      .where((slot) => slot.courseCode == classes.first.courseCode)
      .toList();
  if (mine.isEmpty) return null;
  final faculty = {
    for (final slot in mine) _norm(slot.faculty.split(' - ').first),
  };
  final slotTokens = {for (final slot in mine) ..._slots(slot.slot)};
  final byFaculty = classes
      .where((c) => faculty.contains(_norm(c.faculty)))
      .toList();
  if (byFaculty.length == 1) return byFaculty.single;
  final pool = byFaculty.isEmpty ? classes : byFaculty;
  final bySlot = pool
      .where((c) => _slots(c.slot).intersection(slotTokens).isNotEmpty)
      .toList();
  return bySlot.length == 1 ? bySlot.single : null;
}

DateTime? lectureDate(CourseLecture lecture) => DateTime.tryParse(lecture.date);

/// "Reference Material II" → "Material II".
String materialLabel(String label) =>
    label.replaceFirst(RegExp(r'^reference\s+', caseSensitive: false), '');

/// The lecture to scroll to: the first one today or later.
int? nextLectureIndex(List<CourseLecture> lectures, DateTime now) {
  final today = DateTime(now.year, now.month, now.day);
  for (final (index, lecture) in lectures.indexed) {
    final date = lectureDate(lecture);
    if (date != null && !date.isBefore(today)) return index;
  }
  return null;
}
