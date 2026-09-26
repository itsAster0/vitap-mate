import 'dart:async';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/timetable/presentation/providers/state/timetable_repo.dart';
import 'package:vitapmate/features/timetable/presentation/providers/class_reminder_scheduler.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'timetable_provider.g.dart';

@Riverpod(keepAlive: true)
class Timetable extends _$Timetable {
  Future<TimetableData> _runLoad() async {
    final repo = await ref.watch(timetableRepositoryProvider.future);
    final controller = VtopController<TimetableData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-timetable",
      hooks: VtopHooks(
        // Also from cache: dated reminders need topping up on every
        // start. Not awaited, since it may wait on the calendar.
        onSuccess: (data, {required fromCache}) async {
          unawaited(rescheduleClassReminders(ref.read, timetable: data));
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
        // Also from cache: dated reminders need topping up on every
        // start. Not awaited, since it may wait on the calendar.
        onSuccess: (data, {required fromCache}) async {
          unawaited(rescheduleClassReminders(ref.read, timetable: data));
        },
      ),
    );
    final timetable = await controller.refresh();
    state = AsyncData(timetable);
  }
}
