import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/more/domain/exam_time.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

ExamScheduleRecord _exam(String date, String time) => ExamScheduleRecord(
  serial: '1',
  slot: 'A2',
  courseName: 'Course',
  courseCode: 'CSE1000',
  courseType: 'ETH',
  courseId: 'x',
  examDate: date,
  examSession: 'AN1',
  reportingTime: '01:30 PM',
  examTime: time,
  venue: 'CB-423',
  seatLocation: 'R6C6',
  seatNo: '40',
);

void main() {
  test('parses VTOP date and range start', () {
    final start = examStartOf(_exam('17-Aug-2026', '02:00 PM - 03:30 PM'));
    expect(start, DateTime(2026, 8, 17, 14, 0));
  });

  test('unannounced exams have no date', () {
    expect(examDayOf(_exam('', '')), isNull);
    expect(examStartOf(_exam('-', '')), isNull);
  });

  test('missing time falls back to end of day', () {
    expect(
      examStartOf(_exam('18-Aug-2026', '')),
      DateTime(2026, 8, 18, 23, 59),
    );
  });
}
