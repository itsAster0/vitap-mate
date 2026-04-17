import 'dart:developer';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/attendance/domine/usecases/get_attendance_usecase.dart';
import 'package:vitapmate/features/attendance/domine/usecases/update_attendance_usecase.dart';
import 'package:vitapmate/features/attendance/presentation/providers/state/attendance_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'attendance_provider.g.dart';

@riverpod
class Attendance extends _$Attendance {
  @override
  Future<AttendanceData> build() async {
    var attendanceRepository = await ref.watch(
      attendanceRepositoryProvider.future,
    );
    AttendanceData attendance = await GetAttendanceUsecase(
      attendanceRepository,
    ).call();
    if (attendance.semesterId.isEmpty) {
      await ref.read(vClientProvider.notifier).tryLogin();

      attendance = await _update();
    }
    log("attendance build done");
    return attendance;
  }

  Future<void> updateAttendance() async {
    await ref.read(vClientProvider.notifier).tryLogin();
    var data = await _update();
    state = AsyncData(data);
  }

  Future<AttendanceData> _update() async {
    var repo = await ref.read(attendanceRepositoryProvider.future);
    final featureFlags = await ref.read(featureFlagsControllerProvider.future);
    if (await featureFlags.isEnabled("fetch-attendance")) {
      var data = await UpdateAttendanceUsecase(repo).call();

      return data;
    } else {
      throw FeatureDisabledException("Attendance Feature Disabled");
    }
  }
}
