import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/attendance/domain/attendance_projection.dart';
import 'package:vitapmate/features/attendance/domain/attendance_standing.dart';
import 'package:vitapmate/features/calendar/domain/semester_calendar.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

CalendarEntry _day(
  String date,
  String kind, {
  String note = 'WorkingDay',
  String group = 'General (Semester)',
}) => CalendarEntry(date: date, kind: kind, group: group, note: note);

AcademicCalendarData _calendar(List<CalendarEntry> entries) =>
    AcademicCalendarData(
      entries: entries,
      semesterId: 'S',
      updateTime: BigInt.zero,
    );

AttendanceRecord _record(String type, {int attended = 9, int total = 10}) =>
    AttendanceRecord(
      serial: '1',
      category: '',
      courseName: 'CSE4007 - Digital Image Processing - Embedded $type',
      courseCode: 'CSE4007',
      courseType: type,
      facultyDetail: '',
      classesAttended: '$attended',
      totalClasses: '$total',
      attendancePercentage: '',
      attendenceFatCat: '',
      debarStatus: '',
      courseId: 'C',
    );

TimetableSlot _slot(String day, String start, ClassKind kind) => TimetableSlot(
  serial: '1',
  day: day,
  slot: '',
  courseCode: 'CSE4007',
  courseType: kind == ClassKind.lab ? 'ELA' : 'ETH',
  roomNo: '',
  block: '',
  startTime: start,
  endTime: '',
  name: '',
  kind: kind,
  faculty: '',
  credits: '',
);

// October 2026: Tue 6 .. Sat 10 instructional, Sat 31 is the LAB FAT day.
final _october = SemesterCalendar.of(
  _calendar([
    _day('2026-10-02', 'Holiday', note: 'Mahatma Gandhi Jayanti'),
    _day('2026-10-04', 'Holiday', note: 'Holiday'), // a Sunday
    _day('2026-10-05', 'No Instructional Day', note: 'No Instructional Day'),
    _day('2026-10-06', 'Instructional Day'),
    _day('2026-10-07', 'Instructional Day'),
    _day('2026-10-08', 'Instructional Day'),
    _day('2026-10-09', 'Instructional Day'),
    _day('2026-10-10', 'Instructional Day'),
    _day('2026-10-24', 'Instructional Day'),
    _day('2026-10-31', 'Instructional Day', note: 'LAB FAT'),
    _day('2026-11-03', 'Instructional Day'),
  ]),
);

final _timetable = TimetableData(
  slots: [
    _slot('TUE', '16:00', ClassKind.theory),
    _slot('SAT', '14:00', ClassKind.theory),
    // Two-slot lab: each slot is one class in VTOP's count.
    _slot('SAT', '08:00', ClassKind.lab),
    _slot('SAT', '08:50', ClassKind.lab),
  ],
  courses: const [],
  semesterId: 'S',
  updateTime: BigInt.zero,
);

void main() {
  group('SemesterCalendar', () {
    test('labs stop on the first LAB FAT day, theory carries on', () {
      expect(_october.labFatStart, DateTime(2026, 10, 31));
      final labFat = DateTime(2026, 10, 31);
      expect(_october.holdsClasses(labFat, lab: false), isTrue);
      expect(_october.holdsClasses(labFat, lab: true), isFalse);
      expect(_october.holdsClasses(DateTime(2026, 10, 24), lab: true), isTrue);
    });

    test('holidays and non-instructional days hold no classes', () {
      expect(_october.holdsClasses(DateTime(2026, 10, 2), lab: false), isFalse);
      expect(_october.holdsClasses(DateTime(2026, 10, 5), lab: false), isFalse);
    });

    test('next holiday skips Sundays and past days', () {
      final next = _october.nextHoliday(DateTime(2026, 10, 1));
      expect(next?.date, DateTime(2026, 10, 2));
      expect(next?.name, 'Mahatma Gandhi Jayanti');
      expect(_october.nextHoliday(DateTime(2026, 10, 3)), isNull);
    });

    test('counts instructional days after today', () {
      expect(_october.instructionalDaysLeft(DateTime(2026, 10, 10, 9)), 3);
    });

    test('event days hold classes, CAT and FAT days do not', () {
      final calendar = SemesterCalendar.of(
        _calendar([
          _day(
            '2026-11-14',
            'Instructional Day',
            note: 'Engineering Clinics Expo*',
          ),
          _day(
            '2026-07-14',
            'Instructional Day',
            note: 'First Instructional Day',
          ),
          _day('2026-10-01', 'CAT - II', note: 'Exam Days'),
          _day('2026-11-16', 'Final Assessment Test', note: 'Exam Days'),
        ]),
      );
      bool holds(DateTime d) => calendar.holdsClasses(d, lab: false);
      expect(holds(DateTime(2026, 11, 14)), isTrue);
      expect(holds(DateTime(2026, 7, 14)), isTrue);
      expect(holds(DateTime(2026, 10, 1)), isFalse);
      expect(holds(DateTime(2026, 11, 16)), isFalse);
    });

    test('prefers General (Semester) events over other class groups', () {
      final calendar = SemesterCalendar.of(
        _calendar([
          _day('2026-10-06', 'Holiday', note: 'Holiday'),
          _day('2026-10-06', 'Instructional Day', group: 'Freshers'),
          _day('2026-10-07', 'Instructional Day'),
        ]),
      );
      expect(calendar.holdsClasses(DateTime(2026, 10, 6), lab: false), isFalse);
      expect(calendar.holdsClasses(DateTime(2026, 10, 7), lab: false), isTrue);
    });
  });

  group('exams and coverage', () {
    final calendar = SemesterCalendar.of(
      _calendar([
        _day('2026-08-15', 'Instructional Day'),
        _day('2026-08-17', 'CAT - I', note: 'Exam Days'),
        _day('2026-08-18', 'CAT - I', note: 'Exam Days'),
        _day('2026-09-29', 'CAT - II', note: 'Exam Days'),
        _day('2026-10-01', 'CAT - II', note: 'Exam Days'),
        _day('2026-10-06', 'Instructional Day'),
        _day('2026-11-16', 'Final Assessment Test', note: 'Exam Days'),
      ]),
    );

    test('next exam is the first one after the next class day', () {
      expect(calendar.nextExam(DateTime(2026, 8, 1))?.name, 'CAT-I');
      // From the 16th the next class day is 6 Oct, after both CATs.
      expect(calendar.nextExam(DateTime(2026, 8, 16))?.name, 'FAT');
      // The test calendar has no class days between CAT-I and CAT-II.
      expect(calendar.nextExam(DateTime(2026, 8, 17))?.name, 'FAT');
      expect(calendar.nextExam(DateTime(2026, 9, 30))?.name, 'FAT');
      expect(
        calendar.nextExam(DateTime(2026, 9, 30))?.start,
        DateTime(2026, 11, 16),
      );
      expect(calendar.nextExam(DateTime(2026, 10, 7)), isNull);
    });

    test('days outside the calendar fall back to the timetable', () {
      expect(calendar.classesOn(DateTime(2026, 8, 1), lab: false), isTrue);
      expect(calendar.classesOn(DateTime(2026, 12, 20), lab: false), isTrue);
      // Inside the range: an exam day, and an unlisted day.
      expect(calendar.classesOn(DateTime(2026, 8, 17), lab: false), isFalse);
      expect(calendar.classesOn(DateTime(2026, 8, 16), lab: false), isFalse);
      expect(calendar.entryOn(DateTime(2026, 9, 29))?.kind, 'CAT - II');
    });
  });

  group('AttendanceProjection', () {
    AttendanceProjection project(String type, DateTime now) =>
        AttendanceProjection.of(
          record: _record(type),
          timetable: _timetable,
          calendar: _october,
          now: now,
        );

    test('theory counts Tuesdays and Saturdays, including the LAB FAT day', () {
      // Tue 6, Sat 10, Sat 24, Sat 31, Tue 3 Nov.
      expect(project('ETH', DateTime(2026, 10, 6, 8)).left, 5);
    });

    test('labs count two slots per Saturday until the LAB FAT day', () {
      // Sat 10 and Sat 24, two slots each; Sat 31 is the LAB FAT.
      expect(project('ELA', DateTime(2026, 10, 6, 8)).left, 4);
    });

    test("today's classes count only until they start", () {
      expect(project('ETH', DateTime(2026, 10, 6, 15, 59)).left, 5);
      expect(project('ETH', DateTime(2026, 10, 6, 16, 0)).left, 4);
    });

    test('counting to an exam stops the day before it starts', () {
      final left = AttendanceProjection.of(
        record: _record('ETH'),
        timetable: _timetable,
        calendar: _october,
        now: DateTime(2026, 10, 6, 8),
        until: DateTime(2026, 10, 24),
      ).left;
      // Tue 6 and Sat 10 only.
      expect(left, 2);
    });

    test('nothing left after the last class day', () {
      expect(project('ETH', DateTime(2026, 11, 4)).left, 0);
      expect(project('ELA', DateTime(2026, 10, 25)).left, 0);
    });

    test('semester budget and best case', () {
      const p = AttendanceProjection(
        standing: AttendanceStanding(attended: 41, total: 50),
        left: 18,
      );
      // (41 + 18 - 8) / 68 = 75.0%; missing 9 gives 73.5%.
      expect(p.canMiss, 8);
      expect(p.mustAttend, 10);
      expect(p.bestPercent, closeTo(86.76, 0.01));
      expect(p.canFinishSafe, isTrue);
    });

    test('a course that cannot reach 75% any more', () {
      const p = AttendanceProjection(
        standing: AttendanceStanding(attended: 5, total: 20),
        left: 4,
      );
      expect(p.canMiss, 0);
      expect(p.mustAttend, isNull);
      expect(p.canFinishSafe, isFalse);
    });
  });
}
