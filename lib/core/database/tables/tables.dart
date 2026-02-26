import 'package:drift/drift.dart';

class SemesterTable extends Table {
  TextColumn get semid => text()();
  TextColumn get semName => text()();
  IntColumn get time => integer()();

  @override
  Set<Column> get primaryKey => {semid};
}

class TimetableTable extends Table {
  IntColumn get serial => integer()();
  TextColumn get day => text()();
  TextColumn get slot => text()();
  TextColumn get courseCode => text()();
  TextColumn get courseType => text()();
  TextColumn get courseName => text()();
  TextColumn get roomNo => text()();
  TextColumn get block => text()();
  TextColumn get startTime => text()();
  TextColumn get endTime => text()();
  BoolColumn get isLab => boolean().nullable()();
  TextColumn get faculty => text().nullable()();
  TextColumn get semId => text().references(SemesterTable, #semid)();
  IntColumn get time => integer()();

  @override
  List<Set<Column>>? get uniqueKeys => [
    {semId, startTime, day},
  ];
}

class FullAttendanceTable extends Table {
  IntColumn get serial => integer()();
  TextColumn get date => text()();
  TextColumn get slot => text()();
  TextColumn get dayTime => text()();
  TextColumn get status => text()();
  TextColumn get remark => text()();
  TextColumn get courseType => text()();
  TextColumn get courseId => text()();
  TextColumn get semId => text().references(SemesterTable, #semid)();
  IntColumn get time => integer()();
}

class AttendanceTable extends Table {
  IntColumn get serial => integer()();
  TextColumn get category => text()();
  TextColumn get courseName => text()();
  TextColumn get courseCode => text()();
  TextColumn get courseType => text()();
  TextColumn get facultyDetail => text()();
  TextColumn get classesAttended => text()();
  TextColumn get totalClasses => text()();
  TextColumn get attendancePercentage => text()();
  TextColumn get attendenceFatCat => text()();
  TextColumn get debarStatus => text()();
  TextColumn get courseId => text()();
  TextColumn get semId => text().references(SemesterTable, #semid)();
  IntColumn get time => integer()();

  @override
  List<Set<Column>>? get uniqueKeys => [
    {semId, courseId, courseType},
  ];
}

class ExamScheduleTable extends Table {
  IntColumn get serial => integer()();
  TextColumn get slot => text()();
  TextColumn get courseName => text()();
  TextColumn get courseCode => text()();
  TextColumn get courseType => text()();
  TextColumn get courseId => text()();
  TextColumn get examType => text()();
  TextColumn get examDate => text()();
  TextColumn get examSession => text()();
  TextColumn get reportingTime => text()();
  TextColumn get examTime => text()();
  TextColumn get venue => text()();
  TextColumn get seatLocation => text()();
  TextColumn get seatNo => text()();
  TextColumn get semId => text().references(SemesterTable, #semid)();
  IntColumn get time => integer()();

  @override
  List<Set<Column>>? get uniqueKeys => [
    {courseCode, examDate, examTime, semId, courseType},
  ];
}

class MarksTable extends Table {
  IntColumn get serial => integer()();
  TextColumn get courseCode => text()();
  TextColumn get courseTitle => text()();
  TextColumn get courseType => text()();
  TextColumn get faculty => text()();
  TextColumn get slot => text()();
  TextColumn get marks => text()();
  TextColumn get semId => text().references(SemesterTable, #semid)();
  IntColumn get time => integer()();

  @override
  Set<Column> get primaryKey => {semId, courseCode, courseType, serial, marks};
}

class GradeCourseTable extends Table {
  IntColumn get serial => integer()();
  TextColumn get courseCode => text()();
  TextColumn get courseTitle => text()();
  TextColumn get courseType => text()();
  TextColumn get gradingType => text()();
  TextColumn get grandTotal => text()();
  TextColumn get grade => text()();
  TextColumn get courseId => text()();
  TextColumn get semId => text().references(SemesterTable, #semid)();
  IntColumn get time => integer()();

  @override
  Set<Column> get primaryKey => {semId, courseId};
}

class GradeDetailTable extends Table {
  TextColumn get semId => text().references(SemesterTable, #semid)();
  TextColumn get courseId => text()();
  TextColumn get classNumber => text()();
  TextColumn get classCourseType => text()();
  TextColumn get grandTotal => text()();
  IntColumn get serial => integer()();
  TextColumn get markTitle => text()();
  TextColumn get maxMark => text()();
  TextColumn get weightage => text()();
  TextColumn get status => text()();
  TextColumn get scoredMark => text()();
  TextColumn get weightageMark => text()();
  TextColumn get gradeRanges => text().withDefault(const Constant("[]"))();
  IntColumn get time => integer()();

  @override
  Set<Column> get primaryKey => {semId, courseId, serial, markTitle};
}

class GradeHistoryCacheTable extends Table {
  IntColumn get id => integer().withDefault(const Constant(1))();
  TextColumn get payload => text()();
  IntColumn get time => integer()();

  @override
  Set<Column> get primaryKey => {id};
}
