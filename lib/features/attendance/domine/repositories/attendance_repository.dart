import 'package:vitapmate/src/api/vtop/types.dart';

abstract class AttendanceRepository {
  Future<void> saveAttendanceToStoarge(AttendanceData attendanceEntity);
  Future<AttendanceData> getAttendanceFromStorage();
  Future<void> updateFullAttendance(String courseType, String courseId);
  Future<void> saveFullAttendanceToStoarge(
    FullAttendanceData fullattendanceEntity,
    String courseId,
    String courseType,
  );
  Future<FullAttendanceData> getFullAttendanceFromStorage(
    String courseId,
    String courseType,
  );
  Future<void> updateAttendance();
}
