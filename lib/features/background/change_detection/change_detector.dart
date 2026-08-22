import 'package:vitapmate/src/api/vtop/types.dart';

class DataChangeSummary {
  final String title;
  final String body;

  const DataChangeSummary({required this.title, required this.body});
}

const int _maxDetailLines = 6;

DataChangeSummary? compareAttendance(
  AttendanceData? previous,
  AttendanceData current, {
  double threshold = 75,
}) {
  if (previous == null || previous.updateTime == current.updateTime) {
    return null;
  }

  String keyOf(AttendanceRecord r) => '${r.courseId}|${r.courseType}';
  final previousByKey = {for (final r in previous.records) keyOf(r): r};

  final crossedLines = <String>[];
  final changedLines = <String>[];

  for (final record in current.records) {
    final previousRecord = previousByKey[keyOf(record)];
    final currentPercentage = _parsePercentage(record.attendancePercentage);
    if (currentPercentage == null) continue;

    if (previousRecord == null) {
      changedLines.add('${record.courseCode}: ${_format(currentPercentage)}%');
      continue;
    }

    final previousPercentage = _parsePercentage(
      previousRecord.attendancePercentage,
    );
    if (previousPercentage == null ||
        previousPercentage == currentPercentage) {
      continue;
    }

    final decreased = currentPercentage < previousPercentage;
    if (!decreased) continue;

    final line =
        '${record.courseCode}: ${_format(previousPercentage)}% \u2192 '
        '${_format(currentPercentage)}%';

    if (previousPercentage >= threshold && currentPercentage < threshold) {
      crossedLines.add(line);
    } else {
      changedLines.add(line);
    }
  }

  if (crossedLines.isEmpty && changedLines.isEmpty) return null;

  return DataChangeSummary(
    title: crossedLines.isNotEmpty
        ? 'Attendance below ${_format(threshold)}%'
        : 'Attendance updated',
    body: _join([...crossedLines, ...changedLines]),
  );
}

DataChangeSummary? compareMarks(MarksData? previous, MarksData current) {
  if (previous == null || previous.updateTime == current.updateTime) {
    return null;
  }

  String courseKeyOf(MarksRecord r) => '${r.coursecode}|${r.coursetype}';
  String entryKeyOf(MarksRecordEach m) =>
      m.serial.isNotEmpty ? m.serial : m.markstitle;

  final previousCourses = {
    for (final c in previous.records) courseKeyOf(c): c,
  };
  final lines = <String>[];

  for (final course in current.records) {
    final previousCourse = previousCourses[courseKeyOf(course)];

    if (previousCourse == null) {
      if (course.marks.isNotEmpty) {
        lines.add('${course.coursecode}: ${course.marks.length} entries');
      }
      continue;
    }

    final previousEntries = {
      for (final m in previousCourse.marks) entryKeyOf(m): m,
    };
    final newTitles = <String>[];
    var updatedCount = 0;

    for (final mark in course.marks) {
      final previousMark = previousEntries[entryKeyOf(mark)];
      if (previousMark == null) {
        if (newTitles.length < 3) newTitles.add(mark.markstitle);
      } else if (previousMark.scoredmark != mark.scoredmark ||
          previousMark.status != mark.status) {
        updatedCount++;
      }
    }

    if (newTitles.isNotEmpty) {
      lines.add('${course.coursecode}: ${newTitles.join(', ')}');
    } else if (updatedCount > 0) {
      lines.add('${course.coursecode}: marks revised');
    }
  }

  if (lines.isEmpty) return null;
  return DataChangeSummary(title: 'New marks published', body: _join(lines));
}

DataChangeSummary? compareTimetable(
  TimetableData? previous,
  TimetableData current,
) {
  if (previous == null || previous.updateTime == current.updateTime) {
    return null;
  }

  String signatureOf(TimetableSlot s) =>
      '${s.day}|${s.startTime}|${s.endTime}|${s.slot}|${s.courseCode}'
      '|${s.block}|${s.roomNo}';

  final previousSignatures = previous.slots.map(signatureOf).toSet();
  final currentSignatures = current.slots.map(signatureOf).toSet();

  final added = current.slots
      .where((s) => !previousSignatures.contains(signatureOf(s)))
      .toList();
  final removed = previous.slots
      .where((s) => !currentSignatures.contains(signatureOf(s)))
      .toList();

  if (added.isEmpty && removed.isEmpty) return null;

  final lines = <String>[
    for (final s in added)
      '+ ${s.courseCode} \u00b7 ${s.day} ${s.startTime}'
          '${s.roomNo.trim().isEmpty ? '' : ' \u00b7 ${s.block}${s.roomNo}'}',
    for (final s in removed)
      '- ${s.courseCode} \u00b7 ${s.day} ${s.startTime}',
  ];

  return DataChangeSummary(title: 'Timetable changed', body: _join(lines));
}

DataChangeSummary? compareExamSchedule(
  ExamScheduleData? previous,
  ExamScheduleData current,
) {
  if (previous == null || previous.updateTime == current.updateTime) {
    return null;
  }

  String keyOf(String examType, ExamScheduleRecord r) =>
      '${r.courseId}|$examType';

  final previousExams = <String, ExamScheduleRecord>{
    for (final group in previous.exams)
      for (final r in group.records) keyOf(group.examType, r): r,
  };

  final lines = <String>[];

  for (final group in current.exams) {
    for (final exam in group.records) {
      final previousExam = previousExams[keyOf(group.examType, exam)];

      if (previousExam == null) {
        final venue = exam.venue.trim();
        lines.add(
          '${exam.courseCode} ${group.examType}: ${exam.examDate}'
          '${exam.examTime.trim().isEmpty ? '' : ' ${exam.examTime}'}'
          '${venue.isEmpty ? '' : ' \u00b7 $venue'}',
        );
      } else if (previousExam.examDate != exam.examDate ||
          previousExam.examTime != exam.examTime ||
          previousExam.reportingTime != exam.reportingTime ||
          previousExam.venue != exam.venue ||
          previousExam.seatLocation != exam.seatLocation ||
          previousExam.seatNo != exam.seatNo) {
        lines.add('${exam.courseCode} ${group.examType}: details updated');
      }
    }
  }

  if (lines.isEmpty) return null;
  return DataChangeSummary(
    title: 'Exam schedule updated',
    body: _join(lines),
  );
}

double? _parsePercentage(String value) {
  final cleaned = value.trim().replaceAll('%', '');
  if (cleaned.isEmpty) return null;
  return double.tryParse(cleaned);
}

String _format(double value) {
  final fixed = value.toStringAsFixed(1);
  return fixed.endsWith('.0') ? fixed.substring(0, fixed.length - 2) : fixed;
}

String _join(List<String> lines) {
  if (lines.length <= _maxDetailLines) return lines.join('\n');
  return '${lines.take(_maxDetailLines).join('\n')}'
      '\n+${lines.length - _maxDetailLines} more';
}
