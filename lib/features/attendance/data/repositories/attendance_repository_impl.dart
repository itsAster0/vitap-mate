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
}

class FullAttendanceRepository extends CachedRepository<FullAttendanceData> {
  FullAttendanceRepository({
    required this.semid,
    required this.courseType,
    required this.courseId,
    required AttendanceDataSource dataSource,
  }) : _dataSource = dataSource;

  final String semid;
  final String courseType;
  final String courseId;
  final AttendanceDataSource _dataSource;

  @override
  Future<FullAttendanceData?> loadCache() async {
    final attendance = await _dataSource.getFullAttendance(
      semid,
      courseId,
      courseType,
    );
    if (attendance.semesterId.isEmpty) return null;
    return attendance;
  }

  @override
  Future<void> saveCache(FullAttendanceData data) {
    return _dataSource.saveFullAttendance(data, semid, courseType, courseId);
  }

  @override
  Future<FullAttendanceData> fetchRemote() {
    return _dataSource.fetchFullAttendance(semid, courseType, courseId);
  }
}
