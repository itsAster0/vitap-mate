import 'package:vitapmate/features/attendance/data/datasources/local_data_source.dart';
import 'package:vitapmate/features/attendance/data/datasources/remote_data_source.dart';
import 'package:vitapmate/features/attendance/domine/repositories/attendance_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class AttendanceRepositoryImpl extends AttendanceRepository {
  AttendanceLocalDataSource localDataSource;
  AttendanceRemoteDataSource remoteDataSource;
  String semid;
  AttendanceRepositoryImpl({
    required this.semid,
    required this.remoteDataSource,
    required this.localDataSource,
  });

  @override
  Future<AttendanceData> getAttendanceFromStorage() async {
    var attendance = await localDataSource.getAttendance(semid);
    return attendance;
  }

  @override
  Future<FullAttendanceData> getFullAttendanceFromStorage(
    String courseType,
    String courseId,
  ) async {
    var attendance = await localDataSource.getFullAttendance(
      semid,
      courseType,
      courseId,
    );
    return attendance;
  }

  @override
  Future<void> saveAttendanceToStoarge(AttendanceData attendanceEntity) async {
    await localDataSource.saveAttendance(attendanceEntity, semid);
  }

  @override
  Future<void> saveFullAttendanceToStoarge(
    FullAttendanceData fullattendanceEntity,
    String courseType,
    String courseId,
  ) async {
    await localDataSource.saveFullAttendance(
      fullattendanceEntity,
      semid,
      courseType,
      courseId,
    );
  }

  @override
  Future<void> updateAttendance() async {
    var attendance = await remoteDataSource.fetchAttendanceFromRemote(semid);
    if (attendance.records.isEmpty) return;
    saveAttendanceToStoarge(attendance);
  }

  @override
  Future<void> updateFullAttendance(String courseType, String courseId) async {
    var attendance = await remoteDataSource.fetchFullAttendanceFromRemote(
      semid,
      courseType,
      courseId,
    );
    saveFullAttendanceToStoarge(attendance, courseId, courseType);
  }
}
