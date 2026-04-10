import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/services/service_layer.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'attendance_provider.g.dart';

@riverpod
class Attendance extends _$Attendance {
  @override
  Future<AttendanceData> build() async {
    final services = await ref.watch(appServicesProvider.future);
    AttendanceData attendance =
        await services.vtopDataRepository.loadAttendance();
    if (attendance.semesterId.isEmpty) {
      attendance = await _update();
    }
    return attendance;
  }

  Future<void> updateAttendance() async {
    var data = await _update();
    state = AsyncData(data);
  }

  Future<AttendanceData> _update() async {
    final services = await ref.read(appServicesProvider.future);
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("fetch-attendance");
    if (feature.on && feature.value) {
      return services.vtopDataRepository.loadAttendance(refresh: true);
    } else {
      throw FeatureDisabledException("Attendance Feature Disabled");
    }
  }
}
