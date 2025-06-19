import 'dart:developer';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/attendance/domine/usecases/get_full_attendance_usecase.dart';
import 'package:vitapmate/features/attendance/domine/usecases/update_ful_attendance.dart';
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
    FullAttendanceData attendance =
        await GetFullAttendanceUsecase(
          attendanceRepository,
          _courseType,
          _courseId,
        ).call();
    if (attendance.semesterId.isEmpty) {
      await ref.read(vClientProvider.notifier).tryLogin();

      attendance = await _update();
    } else {
      Future.microtask(() async {
        try {
          await updateAttendance();
        } catch (e) {
          log("$e", level: 900);
        }
      });
    }
    log("full attendace build done $courseId $courseType ");
    return attendance;
  }

  Future<void> updateAttendance() async {
    await ref.read(vClientProvider.notifier).tryLogin();
    var data = await _update();
    state = AsyncData(data);
  }

  Future<FullAttendanceData> _update() async {
    var repo = await ref.read(attendanceRepositoryProvider.future);
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("fetch-full-attendance");
    if (feature.on && feature.value) {
      var data =
          await UpdateFullAttendanceUsecase(
            repo,
            _courseType,
            _courseId,
          ).call();
      return data;
    } else {
      throw FeatureDisabledException("Full Attendance Feature Disabled");
    }
  }
}
