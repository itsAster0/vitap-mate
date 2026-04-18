import 'dart:developer';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/more/presentation/providers/state/exam_schedule.dart';
import 'package:vitapmate/services/exam_reminder_notification_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'exam_schedule.g.dart';

@riverpod
class ExamSchedule extends _$ExamSchedule {
  Future<ExamScheduleData> _runLoad() async {
    final repo = await ref.watch(examScheduleRepositoryProvider.future);
    final controller = VtopController<ExamScheduleData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-exam-schedule",
      hooks: VtopHooks(
        onSuccess: (data, {required fromCache}) async {
          await ExamReminderNotificationService.syncFromExamSchedule(data);
        },
      ),
    );
    return controller.load();
  }

  @override
  Future<ExamScheduleData> build() async {
    final data = await _runLoad();
    log("exam schedule build done");
    return data;
  }

  Future<void> updatexamschedule() async {
    final repo = await ref.read(examScheduleRepositoryProvider.future);
    final controller = VtopController<ExamScheduleData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-exam-schedule",
      hooks: VtopHooks(
        onSuccess: (data, {required fromCache}) async {
          await ExamReminderNotificationService.syncFromExamSchedule(data);
        },
      ),
    );
    final examSchedule = await controller.refresh();
    state = AsyncData(examSchedule);
  }
}
