import 'package:drift/drift.dart';
import 'package:vitapmate/core/database/database.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/features/attendance/data/models/attendance_model.dart';
import 'package:vitapmate/features/attendance/data/models/full_attendance_model.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class AttendanceLocalDataSource {
  final AppDatabase _db;
  final AsyncQueue _globalAsyncQueue;
  AttendanceLocalDataSource(this._db, this._globalAsyncQueue);

  Future<FullAttendanceData> getFullAttendance(
    String semid,
    String courseId,
    String courseType,
  ) async {
    final allrows = await _globalAsyncQueue.run(
      "fromStorage_fullattendance_${semid}_${courseType}_$courseId",
      () =>
          (_db.select(_db.fullAttendanceTable)..where(
            (tbl) =>
                tbl.semId.equals(semid) &
                tbl.courseId.equals(courseId) &
                tbl.courseType.equals(courseType),
          )).get(),
    );
    return FullAttendanceModel.toEntityFromLocal(allrows, courseId, courseType);
  }

  Future<void> saveFullAttendance(
    FullAttendanceData attendance,
    String semid,
    String courseType,
    String courseId,
  ) async {
    await _globalAsyncQueue.run(
      "toStorage_fullattendance_$semid",
      () => (_db.batch((batch) {
        batch.deleteWhere(
          _db.fullAttendanceTable,
          (tbl) =>
              tbl.semId.equals(semid) &
              tbl.courseId.equals(courseId) &
              tbl.courseType.equals(courseType),
        );
        batch.insertAll(_db.fullAttendanceTable, [
          for (var i in attendance.records)
            FullAttendanceTableCompanion.insert(
              serial: int.parse(i.serial),
              date: i.date,
              dayTime: i.dayTime,
              remark: i.remark,
              semId: semid,
              slot: i.slot,
              status: i.status,
              time: attendance.updateTime.toInt(),
              courseId: courseId,
              courseType: courseType,
            ),
        ]);
      })),
    );
  }

  Future<void> saveAttendance(AttendanceData attendance, String semid) async {
    await _globalAsyncQueue.run(
      "toStorage_attendance_$semid",
      () => (_db.batch((batch) {
        batch.deleteWhere(
          _db.attendanceTable,
          (tbl) => tbl.semId.equals(semid),
        );
        batch.insertAll(_db.attendanceTable, [
          for (var i in attendance.records)
            AttendanceTableCompanion.insert(
              serial: int.parse(i.serial),
              category: i.category,
              courseName: i.courseName,
              courseCode: i.courseCode,
              courseType: i.courseType,
              facultyDetail: i.facultyDetail,
              classesAttended: i.classesAttended,
              totalClasses: i.totalClasses,
              attendancePercentage: i.attendancePercentage,
              attendenceFatCat: i.attendenceFatCat,
              debarStatus: i.debarStatus,
              courseId: i.courseId,
              semId: attendance.semesterId,
              time: attendance.updateTime.toInt(),
            ),
        ]);
      })),
    );
  }

  Future<AttendanceData> getAttendance(String semid) async {
    final allrow = await _globalAsyncQueue.run(
      "fromStorage_attendance_$semid",
      () =>
          (_db.select(_db.attendanceTable)
            ..where((tbl) => tbl.semId.equals(semid))).get(),
    );

    return AttendanceModel.toEntityFromLocal(allrow);
  }
}
