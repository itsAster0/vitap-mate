import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/services/service_layer.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
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
    final services = await ref.watch(appServicesProvider.future);
    FullAttendanceData attendance = await services.vtopDataRepository
        .loadFullAttendance(courseType: _courseType, courseId: _courseId);
    if (attendance.semesterId.isEmpty) {
      attendance = await _update();
    }
    return attendance;
  }

  Future<void> updateAttendance() async {
    var data = await _update();
    state = AsyncData(data);
  }

  Future<FullAttendanceData> _update() async {
    final services = await ref.read(appServicesProvider.future);
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("fetch-full-attendance");
    if (feature.on && feature.value) {
      return services.vtopDataRepository.loadFullAttendance(
        courseType: _courseType,
        courseId: _courseId,
        refresh: true,
      );
    } else {
      throw FeatureDisabledException("Full Attendance Feature Disabled");
    }
  }
}
