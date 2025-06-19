import 'dart:developer';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/more/domine/usecases/get_exam_schedule.dart';
import 'package:vitapmate/features/more/domine/usecases/update_exam_schedule.dart';
import 'package:vitapmate/features/more/presentation/providers/state/exam_schedule.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'exam_schedule.g.dart';

@riverpod
class ExamSchedule extends _$ExamSchedule {
  @override
  Future<ExamScheduleData> build() async {
    var repo = await ref.watch(examScheduleRepositoryProvider.future);
    ExamScheduleData data = await GetExamScheduleUsecase(repo).call();
    if (data.semesterId.isEmpty) {
      await ref.read(vClientProvider.notifier).tryLogin();
      data = await _update();
    } else {
      Future.microtask(() async {
        try {
          await updatexamschedule();
        } catch (e) {
          log("$e", level: 900);
        }
      });
    }
    log("exam schedule Build done");
    return data;
  }

  Future<void> updatexamschedule() async {
    await ref.read(vClientProvider.notifier).tryLogin();
    ExamScheduleData data = await _update();
    state = AsyncData(data);
  }

  Future<ExamScheduleData> _update() async {
    var repo = await ref.read(examScheduleRepositoryProvider.future);
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("fetch-exam-schedule");
    if (feature.on && feature.value) {
      var data = await UpdateExamScheduleUsecase(repo).call();
      return data;
    } else {
      throw FeatureDisabledException("Exan schedule Feature Disabled");
    }
  }
}
