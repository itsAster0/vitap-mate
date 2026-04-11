import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/services/service_layer.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/services/exam_reminder_notification_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'exam_schedule.g.dart';

@riverpod
class ExamSchedule extends _$ExamSchedule {
  @override
  Future<ExamScheduleData> build() async {
    final services = await ref.watch(appServicesProvider.future);
    ExamScheduleData data =
        await services.vtopDataRepository.loadExamSchedule();
    if (data.semesterId.isEmpty && ref.watch(autoRefreshOnOpenProvider)) {
      data = await _update();
    }
    await ExamReminderNotificationService.syncFromExamSchedule(data);
    return data;
  }

  Future<void> updatexamschedule() async {
    ExamScheduleData data = await _update();
    state = AsyncData(data);
    await ExamReminderNotificationService.syncFromExamSchedule(data);
  }

  Future<ExamScheduleData> _update() async {
    final services = await ref.read(appServicesProvider.future);
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("fetch-exam-schedule");
    if (feature.on && feature.value) {
      return services.vtopDataRepository.loadExamSchedule(refresh: true);
    } else {
      throw FeatureDisabledException("Exan schedule Feature Disabled");
    }
  }
}
