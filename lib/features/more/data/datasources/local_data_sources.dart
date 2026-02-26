import 'dart:convert';
import 'package:drift/drift.dart';
import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/features/more/data/models/exams_schedule_model.dart';
import 'package:vitapmate/features/more/data/models/grade_history_model.dart';
import 'package:vitapmate/features/more/data/models/grades_model.dart';
import 'package:vitapmate/features/more/data/models/marks_model.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class MarksLocalDataSource {
  final AppDatabase _db;
  final GlobalAsyncQueue _globalAsyncQueue;
  MarksLocalDataSource(this._db, this._globalAsyncQueue);

  Future<MarksData> getMarks(String semid) async {
    final allRows = await _globalAsyncQueue.run(
      "fromStroage_marks_$semid",
      () =>
          (_db.select(_db.marksTable)
            ..where((tbl) => tbl.semId.equals(semid))).get(),
    );
    if (allRows.isEmpty) {
      return MarksData(records: [], semesterId: "", updateTime: BigInt.from(0));
    }
    var k = MarksModel.toEntityFromLocal(allRows);
    return k;
  }

  Future<void> saveMarks(MarksData data, String semid) async {
    await _globalAsyncQueue.run(
      "toStroage_marks_$semid",
      () => (_db.batch((batch) {
        batch.deleteWhere(_db.marksTable, (tbl) => tbl.semId.equals(semid));
        batch.insertAll(_db.marksTable, [
          for (var i in data.records)
            for (var m in i.marks)
              MarksTableCompanion.insert(
                serial: int.parse(i.serial),
                courseCode: i.coursecode,
                courseTitle: i.coursetitle,
                courseType: i.coursetype,
                faculty: i.faculity,
                slot: i.slot,
                marks: jsonEncode(m.toJson()),
                semId: data.semesterId,
                time: data.updateTime.toInt(),
              ),
        ]);
      })),
    );
  }
}

class ExamScheduleLocalDataSource {
  final AppDatabase _db;
  final GlobalAsyncQueue _globalAsyncQueue;
  ExamScheduleLocalDataSource(this._db, this._globalAsyncQueue);

  Future<ExamScheduleData> getExamSchedule(String semid) async {
    final allRows = await _globalAsyncQueue.run(
      "fromStroage__exam_schedule_$semid",
      () =>
          (_db.select(_db.examScheduleTable)
            ..where((tbl) => tbl.semId.equals(semid))).get(),
    );
    if (allRows.isEmpty) {
      return ExamScheduleData(
        exams: [],
        semesterId: "",
        updateTime: BigInt.from(0),
      );
    }
    return ExamsScheduleModel.toEntityFromLocal(allRows);
  }

  Future<void> saveExamSchedule(ExamScheduleData data, String semid) async {
    await _globalAsyncQueue.run(
      "toStroage_exam_schedule_$semid",
      () => (_db.batch((batch) {
        batch.deleteWhere(
          _db.examScheduleTable,
          (tbl) => tbl.semId.equals(semid),
        );
        batch.insertAll(_db.examScheduleTable, [
          for (var i in data.exams)
            for (final m in i.records)
              ExamScheduleTableCompanion.insert(
                serial: int.parse(m.serial),
                slot: m.slot,
                courseName: m.courseName,
                courseCode: m.courseCode,
                courseType: m.courseType,
                courseId: m.courseId,
                examType: i.examType,
                examDate: m.examDate,
                examSession: m.examSession,
                reportingTime: m.reportingTime,
                examTime: m.examTime,
                venue: m.venue,
                seatLocation: m.seatLocation,
                seatNo: m.seatNo,
                semId: data.semesterId,
                time: data.updateTime.toInt(),
              ),
        ]);
      })),
    );
  }
}

class GradesLocalDataSource {
  final AppDatabase _db;
  final GlobalAsyncQueue _globalAsyncQueue;
  GradesLocalDataSource(this._db, this._globalAsyncQueue);

  Future<GradeViewData> getGradeView(String semid) async {
    final rows = await _globalAsyncQueue.run(
      "fromStorage_grades_view_$semid",
      () =>
          (_db.select(_db.gradeCourseTable)
            ..where((tbl) => tbl.semId.equals(semid))).get(),
    );
    if (rows.isEmpty) {
      return GradeViewData(
        courses: const [],
        semesters: const [],
        semesterId: "",
        updateTime: BigInt.zero,
      );
    }
    return GradesModel.toViewFromLocal(rows);
  }

  Future<Map<String, GradeDetailsData>> getGradeDetailsMap(String semid) async {
    final rows = await _globalAsyncQueue.run(
      "fromStorage_grades_details_$semid",
      () =>
          (_db.select(_db.gradeDetailTable)
            ..where((tbl) => tbl.semId.equals(semid))).get(),
    );
    if (rows.isEmpty) return {};
    return GradesModel.toDetailsMapFromLocal(rows);
  }

  Future<void> saveGradeView(GradeViewData data, String semid) async {
    await _globalAsyncQueue.run(
      "toStorage_grades_view_$semid",
      () => _db.batch((batch) {
        batch.deleteWhere(
          _db.gradeCourseTable,
          (tbl) => tbl.semId.equals(semid),
        );
        batch.insertAll(_db.gradeCourseTable, [
          for (final c in data.courses)
            GradeCourseTableCompanion.insert(
              serial: int.tryParse(c.serial) ?? 0,
              courseCode: c.courseCode,
              courseTitle: c.courseTitle,
              courseType: c.courseType,
              gradingType: c.gradingType,
              grandTotal: c.grandTotal,
              grade: c.grade,
              courseId: c.courseId,
              semId: data.semesterId,
              time: data.updateTime.toInt(),
            ),
        ]);
      }),
    );
  }

  Future<void> saveGradeDetails(GradeDetailsData data, String semid) async {
    await _globalAsyncQueue.run(
      "toStorage_grades_details_${semid}_${data.courseId}",
      () => _db.batch((batch) {
        batch.deleteWhere(
          _db.gradeDetailTable,
          (tbl) => tbl.semId.equals(semid) & tbl.courseId.equals(data.courseId),
        );
        batch.insertAll(_db.gradeDetailTable, [
          for (final m in data.marks)
            GradeDetailTableCompanion.insert(
              semId: data.semesterId,
              courseId: data.courseId,
              classNumber: data.classNumber,
              classCourseType: data.classCourseType,
              grandTotal: data.grandTotal,
              serial: int.tryParse(m.serial) ?? 0,
              markTitle: m.markTitle,
              maxMark: m.maxMark,
              weightage: m.weightage,
              status: m.status,
              scoredMark: m.scoredMark,
              weightageMark: m.weightageMark,
              gradeRanges: Value(
                jsonEncode([for (final r in data.gradeRanges) r.toJson()]),
              ),
              time: data.updateTime.toInt(),
            ),
        ]);
      }),
    );
  }
}

class GradeHistoryLocalDataSource {
  final AppDatabase _db;
  final GlobalAsyncQueue _globalAsyncQueue;
  GradeHistoryLocalDataSource(this._db, this._globalAsyncQueue);

  Future<GradeHistoryData> getGradeHistory() async {
    final row = await _globalAsyncQueue.run(
      "fromStorage_grade_history",
      () => _db.select(_db.gradeHistoryCacheTable).getSingleOrNull(),
    );
    if (row == null) {
      return GradeHistoryData(
        student: GradeHistoryStudentInfo(
          regNo: "",
          name: "",
          programmeBranch: "",
          programmeMode: "",
          studySystem: "",
          gender: "",
          yearJoined: "",
          eduStatus: "",
          school: "",
          campus: "",
        ),
        records: const [],
        cgpa: GradeHistoryCgpa(
          creditsRegistered: "",
          creditsEarned: "",
          cgpa: "",
          sGrades: "",
          aGrades: "",
          bGrades: "",
          cGrades: "",
          dGrades: "",
          eGrades: "",
          fGrades: "",
          nGrades: "",
        ),
        updateTime: BigInt.zero,
      );
    }
    return GradeHistoryModel.fromLocal(row);
  }

  Future<void> saveGradeHistory(GradeHistoryData data) async {
    await _globalAsyncQueue.run(
      "toStorage_grade_history",
      () => _db
          .into(_db.gradeHistoryCacheTable)
          .insertOnConflictUpdate(
            GradeHistoryCacheTableCompanion(
              id: const Value(1),
              payload: Value(jsonEncode(data.toJson())),
              time: Value(data.updateTime.toInt()),
            ),
          ),
    );
  }
}
