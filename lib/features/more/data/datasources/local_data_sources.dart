import 'dart:convert';
import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/features/more/data/models/exams_schedule_model.dart';
import 'package:vitapmate/features/more/data/models/marks_model.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class MarksLocalDataSource {
  final AppDatabase _db;
  final GlobalAsyncQueue _globalAsyncQueue;
  MarksLocalDataSource(this._db, this._globalAsyncQueue);

  Future<MarksData> getMarks(String semid) async {
    final allRows = await _globalAsyncQueue.run(
      "fromStroage_vtop_marks_$semid",
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
