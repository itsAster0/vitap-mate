import 'dart:developer';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/attendance/presentation/providers/state/attendance_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'attendance_provider.g.dart';

@Riverpod(keepAlive: true)
class Attendance extends _$Attendance {
  Future<AttendanceData> _runLoad() async {
    final repo = await ref.watch(attendanceRepositoryProvider.future);
    final controller = VtopController<AttendanceData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-attendance",
    );
    return controller.load();
  }

  @override
  Future<AttendanceData> build() async {
    final attendance = await _runLoad();
    log("attendance build done");
    return attendance;
  }

  Future<void> updateAttendance() async {
    final repo = await ref.read(attendanceRepositoryProvider.future);
    final controller = VtopController<AttendanceData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-attendance",
    );
    final attendance = await controller.refresh();
    state = AsyncData(attendance);
  }
}
