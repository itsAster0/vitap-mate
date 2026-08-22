import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/background/change_detection/change_detector.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

AttendanceRecord attRecord(
  String courseId,
  String courseCode,
  String percentage, {
  String courseType = 'THEORY',
}) {
  return AttendanceRecord(
    serial: courseId,
    category: '',
    courseName: 'Course $courseCode',
    courseCode: courseCode,
    courseType: courseType,
    facultyDetail: 'FAC',
    classesAttended: '10',
    totalClasses: '12',
    attendancePercentage: percentage,
    attendenceFatCat: '',
    debarStatus: '',
    courseId: courseId,
  );
}

AttendanceData attendanceData(
  List<AttendanceRecord> records, {
  BigInt? updateTime,
}) {
  return AttendanceData(
    records: records,
    semesterId: 'VL20262',
    updateTime: updateTime ?? BigInt.one,
  );
}

MarksData marksData(List<MarksRecord> records, {BigInt? updateTime}) {
  return MarksData(
    records: records,
    semesterId: 'VL20262',
    updateTime: updateTime ?? BigInt.one,
  );
}

MarksRecord marksRecord(
  String courseCode,
  List<MarksRecordEach> marks, {
  String courseType = 'THEORY',
}) {
  return MarksRecord(
    serial: courseCode,
    coursecode: courseCode,
    coursetitle: 'Title $courseCode',
    coursetype: courseType,
    faculity: 'FAC',
    slot: 'A1',
    marks: marks,
  );
}

MarksRecordEach markEntry(String serial, String title, String scored) {
  return MarksRecordEach(
    serial: serial,
    markstitle: title,
    maxmarks: '100',
    weightage: '10',
    status: 'P',
    scoredmark: scored,
    weightagemark: '9',
    remark: '',
  );
}

TimetableSlot slot({
  String day = 'MON',
  String startTime = '09:00',
  String endTime = '09:50',
  String slotName = 'A1',
  String courseCode = 'CSE101',
  String block = 'AB1',
  String roomNo = '101',
  String courseType = 'THEORY',
}) {
  return TimetableSlot(
    serial: '$day$startTime$courseCode',
    day: day,
    slot: slotName,
    courseCode: courseCode,
    courseType: courseType,
    roomNo: roomNo,
    block: block,
    startTime: startTime,
    endTime: endTime,
    name: courseCode,
    isLab: false,
    faculty: 'FAC',
    credits: '3.0',
  );
}

TimetableData timetableData(List<TimetableSlot> slots, {BigInt? updateTime}) {
  return TimetableData(
    slots: slots,
    courses: const [],
    semesterId: 'VL20262',
    updateTime: updateTime ?? BigInt.one,
  );
}

ExamScheduleData examData(
  List<PerExamScheduleRecord> exams, {
  BigInt? updateTime,
}) {
  return ExamScheduleData(
    exams: exams,
    semesterId: 'VL20262',
    updateTime: updateTime ?? BigInt.one,
  );
}

ExamScheduleRecord examRecord({
  String courseId = 'C1',
  String courseCode = 'CSE101',
  String examDate = '2026-03-12',
  String venue = 'AB1-101',
  String seatNo = '',
  String examTime = 'FN',
}) {
  return ExamScheduleRecord(
    serial: courseId,
    slot: 'A1',
    courseName: 'Course $courseCode',
    courseCode: courseCode,
    courseType: 'THEORY',
    courseId: courseId,
    examDate: examDate,
    examSession: 'FN',
    reportingTime: '08:30',
    examTime: examTime,
    venue: venue,
    seatLocation: 'AB1',
    seatNo: seatNo,
  );
}

void main() {
  group('compareAttendance', () {
    test('returns null on baseline (no previous data)', () {
      final current = attendanceData([attRecord('1', 'CSE101', '80.0')]);
      expect(compareAttendance(null, current), isNull);
    });

    test('returns null when updateTime unchanged', () {
      final data = attendanceData([attRecord('1', 'CSE101', '80.0')]);
      expect(compareAttendance(data, data), isNull);
    });

    test('returns null when nothing changed', () {
      final previous = attendanceData([
        attRecord('1', 'CSE101', '80.0'),
      ], updateTime: BigInt.one);
      final current = attendanceData([
        attRecord('1', 'CSE101', '80.0'),
      ], updateTime: BigInt.two);
      expect(compareAttendance(previous, current), isNull);
    });

    test('detects percentage drop and ignores rise', () {
      final previous = attendanceData([
        attRecord('1', 'CSE101', '85.7'),
        attRecord('2', 'MAT101', '70.0'),
      ]);
      final current = attendanceData([
        attRecord('1', 'CSE101', '82.3'),
        attRecord('2', 'MAT101', '72.5'),
      ], updateTime: BigInt.two);

      final summary = compareAttendance(previous, current);
      expect(summary, isNotNull);
      expect(summary!.title, 'Attendance updated');
      expect(summary.body, contains('CSE101: 85.7% \u2192 82.3%'));
      expect(summary.body, isNot(contains('MAT101')));
    });

    test('prioritizes threshold crossing in title and ordering', () {
      final previous = attendanceData([
        attRecord('1', 'CSE101', '76.0'),
        attRecord('2', 'MAT101', '90.0'),
      ]);
      final current = attendanceData([
        attRecord('1', 'CSE101', '74.9'),
        attRecord('2', 'MAT101', '88.0'),
      ], updateTime: BigInt.two);

      final summary = compareAttendance(previous, current);
      expect(summary!.title, 'Attendance below 75%');
      expect(summary.body.startsWith('CSE101'), isTrue);
    });

    test('lists newly added course', () {
      final previous = attendanceData([attRecord('1', 'CSE101', '80.0')]);
      final current = attendanceData([
        attRecord('1', 'CSE101', '80.0'),
        attRecord('3', 'PHY101', '95.5'),
      ], updateTime: BigInt.two);

      final summary = compareAttendance(previous, current);
      expect(summary!.body, contains('PHY101'));
    });

    test('supports custom threshold', () {
      final previous = attendanceData([attRecord('1', 'CSE101', '81.0')]);
      final current = attendanceData([
        attRecord('1', 'CSE101', '79.9'),
      ], updateTime: BigInt.two);

      final summary = compareAttendance(previous, current, threshold: 80);
      expect(summary!.title, 'Attendance below 80%');
    });
  });

  group('compareMarks', () {
    test('returns null on baseline or unchanged data', () {
      final current = marksData([
        marksRecord('CSE101', [markEntry('m1', 'CAT-1', '80')]),
      ]);
      expect(compareMarks(null, current), isNull);
      expect(compareMarks(current, current), isNull);
    });

    test('detects new marks entries with titles', () {
      final previous = marksData([
        marksRecord('CSE101', [markEntry('m1', 'CAT-1', '80')]),
      ]);
      final current = marksData([
        marksRecord('CSE101', [
          markEntry('m1', 'CAT-1', '80'),
          markEntry('m2', 'CAT-2', '90'),
        ]),
      ], updateTime: BigInt.two);

      final summary = compareMarks(previous, current);
      expect(summary!.title, 'New marks published');
      expect(summary.body, contains('CSE101: CAT-2'));
    });

    test('detects new course with marks', () {
      final previous = marksData([]);
      final current = marksData([
        marksRecord('MAT201', [markEntry('x1', 'Quiz-1', '45')]),
      ], updateTime: BigInt.two);

      final summary = compareMarks(previous, current);
      expect(summary!.body, contains('MAT201'));
    });

    test('detects revised scores without new entries', () {
      final previous = marksData([
        marksRecord('CSE101', [markEntry('m1', 'CAT-1', '80')]),
      ]);
      final current = marksData([
        marksRecord('CSE101', [markEntry('m1', 'CAT-1', '85')]),
      ], updateTime: BigInt.two);

      final summary = compareMarks(previous, current);
      expect(summary!.body, contains('CSE101: marks revised'));
    });
  });

  group('compareTimetable', () {
    test('returns null on baseline or identical slots', () {
      final data = timetableData([slot()]);
      expect(compareTimetable(null, data), isNull);
      expect(compareTimetable(data, data), isNull);
    });

    test('detects added and removed classes', () {
      final previous = timetableData([slot()]);
      final current = timetableData([
        slot(),
        slot(day: 'TUE', startTime: '10:00', courseCode: 'CSE202'),
      ], updateTime: BigInt.two);
      final removedCase = timetableData([], updateTime: BigInt.from(3));

      final summary = compareTimetable(previous, current);
      expect(summary!.body, contains('+ CSE202'));

      final removal = compareTimetable(current, removedCase);
      expect(removal!.body, contains('- CSE101'));
    });

    test('treats room change as a modification', () {
      final previous = timetableData([slot(roomNo: '101')]);
      final current = timetableData([
        slot(roomNo: '205'),
      ], updateTime: BigInt.two);

      final summary = compareTimetable(previous, current);
      expect(summary, isNotNull);
      expect(summary!.body, contains('+ CSE101'));
      expect(summary.body, contains('- CSE101'));
    });
  });

  group('compareExamSchedule', () {
    test('returns null on baseline or unchanged schedule', () {
      final data = examData([
        PerExamScheduleRecord(records: [examRecord()], examType: 'CAT-1'),
      ]);
      expect(compareExamSchedule(null, data), isNull);
      expect(compareExamSchedule(data, data), isNull);
    });

    test('detects newly published exams with details', () {
      final previous = examData([]);
      final current = examData([
        PerExamScheduleRecord(records: [examRecord()], examType: 'CAT-1'),
      ], updateTime: BigInt.two);

      final summary = compareExamSchedule(previous, current);
      expect(summary!.title, 'Exam schedule updated');
      expect(
        summary.body,
        contains('CSE101 CAT-1: 2026-03-12 FN \u00b7 AB1-101'),
      );
    });

    test('detects venue change as details update', () {
      final previous = examData([
        PerExamScheduleRecord(records: [examRecord()], examType: 'CAT-1'),
      ]);
      final current = examData([
        PerExamScheduleRecord(
          records: [examRecord(venue: 'AB2-999')],
          examType: 'CAT-1',
        ),
      ], updateTime: BigInt.two);

      final summary = compareExamSchedule(previous, current);
      expect(summary!.body, contains('CSE101 CAT-1: details updated'));
    });
  });
}
