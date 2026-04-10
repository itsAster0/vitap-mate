import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/services/service_layer.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/services/class_reminder_notification_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'timetable_provider.g.dart';

@riverpod
class Timetable extends _$Timetable {
  @override
  Future<TimetableData> build() async {
    final services = await ref.watch(appServicesProvider.future);
    TimetableData timetable = await services.vtopDataRepository.loadTimetable();
    if (timetable.slots.isEmpty) {
      timetable = await _update();
    }
    await ClassReminderNotificationService.syncFromTimetable(timetable);
    return timetable;
  }

  Future<void> updateTimetable() async {
    TimetableData data = await _update();
    state = AsyncData(data);
    await ClassReminderNotificationService.syncFromTimetable(data);
  }

  Future<TimetableData> _update() async {
    final services = await ref.read(appServicesProvider.future);
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("fetch-timetable");
    if (feature.on && feature.value) {
      return services.vtopDataRepository.loadTimetable(refresh: true);
    } else {
      throw FeatureDisabledException("Timetable Feature Disabled");
    }
  }
}
