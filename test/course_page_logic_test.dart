import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/course_page/domain/course_page_logic.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

CoursePageClass section(String faculty, String slot, {String id = 'c'}) =>
    CoursePageClass(
      classId: id,
      erpId: '1',
      classGroup: 'General (Semester)',
      courseCode: 'CSE4007',
      courseTitle: 'Digital Image Processing',
      courseType: 'Embedded Theory',
      slot: slot,
      faculty: faculty,
      facultySchool: 'SENSE',
    );

TimetableData timetable(List<(String, String, String)> slots) => TimetableData(
  slots: [
    for (final (code, slot, faculty) in slots)
      TimetableSlot(
        serial: '1',
        day: 'MON',
        slot: slot,
        courseCode: code,
        courseType: 'ETH',
        roomNo: '315',
        block: 'CB',
        startTime: '08:00',
        endTime: '08:50',
        name: 'Digital Image Processing',
        kind: ClassKind.theory,
        faculty: faculty,
        credits: '3',
      ),
  ],
  courses: const [],
  semesterId: 'AP2026272',
  updateTime: BigInt.zero,
);

void main() {
  test('groups course parts under their code', () {
    final groups = groupCourses(const [
      CoursePageCourse(
        id: '1',
        code: 'CSE4007',
        title: 'DIP',
        courseType: 'ETH',
      ),
      CoursePageCourse(
        id: '2',
        code: 'CSE4007',
        title: 'DIP',
        courseType: 'ELA',
      ),
      CoursePageCourse(
        id: '3',
        code: 'LIB2019',
        title: 'Water',
        courseType: 'TH',
      ),
    ]);
    expect(groups.map((g) => g.code), ['CSE4007', 'LIB2019']);
    expect(groups.first.parts.map((p) => courseTypeLabel(p.courseType)), [
      'Theory',
      'Lab',
    ]);
  });

  test('finds the student\'s section by faculty', () {
    final classes = [
      section('Sucharitha M', 'A1/TA1', id: 'a'),
      section('Kankanala Srinivas', 'B2/TB2', id: 'b'),
    ];
    final mine = myClass(
      classes,
      timetable([('CSE4007', 'B2', 'Kankanala Srinivas - SENSE')]),
    );
    expect(mine?.classId, 'b');
  });

  test('uses the slot when one faculty teaches several sections', () {
    final classes = [
      section('Sucharitha M', 'A1/TA1', id: 'a'),
      section('Sucharitha M', 'A2/TA2', id: 'b'),
    ];
    expect(
      myClass(
        classes,
        timetable([('CSE4007', 'TA2', 'Sucharitha M - SENSE')]),
      )?.classId,
      'b',
    );
    // Not taking the course at all: no guess.
    expect(myClass(classes, timetable([('SWE2009', 'A1', 'X - Y')])), isNull);
  });

  test('picks the next lecture', () {
    const lectures = [
      CourseLecture(
        serial: '1',
        date: '2026-10-01',
        day: '',
        topic: '',
        materials: [],
      ),
      CourseLecture(
        serial: '2',
        date: '2026-10-08',
        day: '',
        topic: '',
        materials: [],
      ),
    ];
    expect(nextLectureIndex(lectures, DateTime(2026, 10, 7, 19)), 1);
    expect(nextLectureIndex(lectures, DateTime(2026, 10, 9)), isNull);
  });
}
