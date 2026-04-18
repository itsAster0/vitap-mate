import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/features/attendance/data/datasources/data_source.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class AttendanceRepository extends CachedRepository<AttendanceData> {
  final AttendanceDataSource _dataSource;
  final String semid;

  AttendanceRepository({
    required this.semid,
    required AttendanceDataSource dataSource,
  }) : _dataSource = dataSource;

  @override
  Future<AttendanceData?> loadCache() async {
    final attendance = await _dataSource.getAttendance(semid);
    if (attendance.semesterId.isEmpty) return null;
    return attendance;
  }

  @override
  Future<void> saveCache(AttendanceData data) async {
    await _dataSource.saveAttendance(data, semid);
  }

  @override
  Future<AttendanceData> fetchRemote() async {
    return _dataSource.fetchAttendance(semid);
  }

  Future<FullAttendanceData> getFullAttendanceFromStorage(
    String courseType,
    String courseId,
  ) {
    return _dataSource.getFullAttendance(semid, courseId, courseType);
  }

  Future<void> saveFullAttendanceToStorage(
    FullAttendanceData fullAttendance,
    String courseType,
    String courseId,
  ) {
    return _dataSource.saveFullAttendance(
      fullAttendance,
      semid,
      courseType,
      courseId,
    );
  }

  Future<void> updateFullAttendance(String courseType, String courseId) async {
    final attendance = await _dataSource.fetchFullAttendance(
      semid,
      courseType,
      courseId,
    );
    await saveFullAttendanceToStorage(attendance, courseType, courseId);
  }
}
