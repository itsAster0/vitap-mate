import 'dart:developer';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/attendance/presentation/providers/state/attendance_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'full_attendance_provider.g.dart';

@riverpod
class FullAttendance extends _$FullAttendance {
  late String _courseType;
  late String _courseId;
  @override
  Future<FullAttendanceData> build(String courseType, String courseId) async {
    _courseType = courseType;
    _courseId = courseId;
    var attendanceRepository = await ref.watch(
      attendanceRepositoryProvider.future,
    );
    FullAttendanceData attendance = await attendanceRepository
        .getFullAttendanceFromStorage(_courseType, _courseId);
    if (attendance.semesterId.isEmpty) {
      await ref.read(vClientProvider.notifier).ensureLogin();

      attendance = await _update();
    }
    log("full attendace build done $courseId $courseType ");
    return attendance;
  }

  Future<void> updateAttendance() async {
    await ref.read(vClientProvider.notifier).ensureLogin();
    var data = await _update();
    state = AsyncData(data);
  }

  Future<FullAttendanceData> _update() async {
    var repo = await ref.read(attendanceRepositoryProvider.future);
    final featureFlags = await ref.read(featureFlagsControllerProvider.future);
    if (await featureFlags.isEnabled("fetch-full-attendance")) {
      await repo.updateFullAttendance(_courseType, _courseId);
      var data = await repo.getFullAttendanceFromStorage(
        _courseType,
        _courseId,
      );
      return data;
    } else {
      throw FeatureDisabledException("Full Attendance Feature Disabled");
    }
  }
}
