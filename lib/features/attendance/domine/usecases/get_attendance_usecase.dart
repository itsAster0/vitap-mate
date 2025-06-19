import 'package:vitapmate/core/utils/usecase.dart';
import 'package:vitapmate/features/attendance/domine/repositories/attendance_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class GetAttendanceUsecase implements Usecase<AttendanceData, void> {
  final AttendanceRepository _attendanceRepository;
  GetAttendanceUsecase(this._attendanceRepository);
  @override
  Future<AttendanceData> call({void params}) async {
    var timetable = await _attendanceRepository.getAttendanceFromStorage();
    return timetable;
  }
}
