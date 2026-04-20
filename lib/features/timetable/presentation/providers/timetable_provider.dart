import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/timetable/presentation/providers/state/timetable_repo.dart';
import 'package:vitapmate/services/class_reminder_notification_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'timetable_provider.g.dart';

@riverpod
class Timetable extends _$Timetable {
  Future<TimetableData> _runLoad() async {
    final repo = await ref.watch(timetableRepositoryProvider.future);
    final controller = VtopController<TimetableData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-timetable",
      hooks: VtopHooks(
        onSuccess: (data, {required fromCache}) async {
          if (!fromCache) {
            await ClassReminderNotificationService.syncFromTimetable(data);
          }
        },
      ),
    );
    return controller.load();
  }

  @override
  Future<TimetableData> build() async {
    final timetable = await _runLoad();
    return timetable;
  }

  Future<void> updateTimetable() async {
    final repo = await ref.read(timetableRepositoryProvider.future);
    final controller = VtopController<TimetableData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-timetable",
      hooks: VtopHooks(
        onSuccess: (data, {required fromCache}) async {
          if (!fromCache) {
            await ClassReminderNotificationService.syncFromTimetable(data);
          }
        },
      ),
    );
    final timetable = await controller.refresh();
    state = AsyncData(timetable);
  }
}
