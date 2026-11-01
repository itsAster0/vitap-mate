import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

CalendarEntry _day(String date, String kind, {String note = 'WorkingDay'}) =>
    CalendarEntry(
      date: date,
      kind: kind,
      group: 'General (Semester)',
      note: note,
    );

// Lab FAT Sat 31 Oct to Fri 6 Nov 2026 (Sunday and Monday off), then
// classes to Sat 14 Nov.
final _calendar = SemesterCalendar.of(
  AcademicCalendarData(
    semesterId: 'S',
    updateTime: BigInt.zero,
    entries: [
      _day('2026-10-28', 'Instructional Day'),
      _day('2026-10-31', 'Instructional Day', note: 'LAB FAT'),
      _day('2026-11-01', 'Holiday', note: ''),
      _day('2026-11-02', 'No Instructional Day', note: ''),
      for (final d in ['03', '04', '05', '06'])
        _day('2026-11-$d', 'Instructional Day', note: 'LAB FAT'),
      _day('2026-11-11', 'Instructional Day'),
      _day('2026-11-14', 'Instructional Day'),
    ],
  ),
);

void main() {
  group('SemesterCalendar.classDay', () {
    test('a lab meets as usual before the Lab FAT', () {
      expect(
        _calendar.classDay(DateTime(2026, 10, 28), lab: true, labWeekdays: {3}),
        ClassDay.held,
      );
    });

    test('a lab on its first Lab FAT day is the Lab FAT', () {
      expect(
        _calendar.classDay(DateTime(2026, 11, 4), lab: true, labWeekdays: {3}),
        ClassDay.labFat,
      );
    });

    test('only the first of two lab days in the window is the Lab FAT', () {
      // Labs on Wednesday and Friday: Wed 4 is the exam, Fri 6 is off.
      const weekdays = {3, 5};
      expect(
        _calendar.classDay(
          DateTime(2026, 11, 4),
          lab: true,
          labWeekdays: weekdays,
        ),
        ClassDay.labFat,
      );
      expect(
        _calendar.classDay(
          DateTime(2026, 11, 6),
          lab: true,
          labWeekdays: weekdays,
        ),
        ClassDay.off,
      );
    });

    test('labs stop after the Lab FAT; theory carries on', () {
      final day = DateTime(2026, 11, 11);
      expect(
        _calendar.classDay(day, lab: true, labWeekdays: {3}),
        ClassDay.off,
      );
      expect(_calendar.classDay(day, lab: false), ClassDay.held);
    });

    test('attendance never counts a Lab FAT', () {
      expect(_calendar.holdsClasses(DateTime(2026, 11, 4), lab: true), false);
    });
  });
}
