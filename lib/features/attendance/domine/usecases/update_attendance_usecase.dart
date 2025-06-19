import 'package:vitapmate/core/utils/usecase.dart';
import 'package:vitapmate/features/attendance/domine/repositories/attendance_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class UpdateAttendanceUsecase implements Usecase<AttendanceData, void> {
  final AttendanceRepository _attendanceRepository;
  UpdateAttendanceUsecase(this._attendanceRepository);
  @override
  Future<AttendanceData> call({void params}) async {
    await _attendanceRepository.updateAttendance();
    var data = await _attendanceRepository.getAttendanceFromStorage();
    return data;
  }
}
