import 'dart:developer';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/timetable/domine/usecases/get_timetable.dart';
import 'package:vitapmate/features/timetable/domine/usecases/update_timetable.dart';
import 'package:vitapmate/features/timetable/presentation/providers/pb_links.dart';
import 'package:vitapmate/features/timetable/presentation/providers/state/timetable_repo.dart';
import 'package:vitapmate/services/class_reminder_notification_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'timetable_provider.g.dart';

@riverpod
class Timetable extends _$Timetable {
  @override
  Future<TimetableData> build() async {
    var timetableRepository = await ref.watch(
      timetableRepositoryProvider.future,
    );

    TimetableData timetable =
        await GetTimetableUsecase(timetableRepository).call();
    if (timetable.slots.isEmpty) {
      await ref.read(vClientProvider.notifier).tryLogin();
      timetable = await _update();
    }
    await ClassReminderNotificationService.syncFromTimetable(timetable);
    log("timetabel Build done");
    return timetable;
  }

  Future<void> updateTimetable() async {
    await ref.read(vClientProvider.notifier).tryLogin();
    TimetableData data = await _update();
    Future.microtask(() async {
      var dataTT = await getTimetablelinkLocal();
      for (var i in dataTT) {
        final vals = i.split("_");
        if (vals.isEmpty) continue;
        if (vals[0] == data.semesterId) {
          await ref.read(createTimetabledbProvider(data).future);
        }
      }
    });
    state = AsyncData(data);
    await ClassReminderNotificationService.syncFromTimetable(data);
  }

  Future<TimetableData> _update() async {
    var repo = await ref.read(timetableRepositoryProvider.future);
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("fetch-timetable");
    if (feature.on && feature.value) {
      var data = await UpdateTimetableUsecase(repo).call();
      return data;
    } else {
      throw FeatureDisabledException("Timetable Feature Disabled");
    }
  }
}
