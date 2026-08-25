import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/attendance/presentation/providers/state/attendance_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'full_attendance_provider.g.dart';

@Riverpod(keepAlive: true)
class FullAttendance extends _$FullAttendance {
  @override
  Future<FullAttendanceData> build(String courseType, String courseId) async {
    final repository = await ref.watch(
      fullAttendanceRepositoryProvider(courseType, courseId).future,
    );
    return VtopController<FullAttendanceData>(
      ref: ref,
      repository: repository,
      featureName: 'fetch-full-attendance',
    ).load();
  }

  Future<void> updateAttendance() async {
    final repository = await ref.read(
      fullAttendanceRepositoryProvider(courseType, courseId).future,
    );
    final attendance = await VtopController<FullAttendanceData>(
      ref: ref,
      repository: repository,
      featureName: 'fetch-full-attendance',
    ).refresh();
    state = AsyncData(attendance);
  }
}
