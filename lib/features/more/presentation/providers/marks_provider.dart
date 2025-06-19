import 'dart:developer';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/more/domine/usecases/get_marks.dart';
import 'package:vitapmate/features/more/domine/usecases/update_marks.dart';
import 'package:vitapmate/features/more/presentation/providers/state/exam_schedule.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'marks_provider.g.dart';

@riverpod
class Marks extends _$Marks {
  @override
  Future<MarksData> build() async {
    var repo = await ref.watch(marksRepositoryProvider.future);
    MarksData data = await GetMarksUsecase(repo).call();
    if (data.records.isEmpty) {
      await ref.read(vClientProvider.notifier).tryLogin();
      data = await _update();
    } else {
      Future.microtask(() async {
        try {
          await updatemarks();
        } catch (e) {
          log("$e", level: 900);
        }
      });
    }
    log("timetabel Build done");
    return data;
  }

  Future<void> updatemarks() async {
    await ref.read(vClientProvider.notifier).tryLogin();
    MarksData data = await _update();
    state = AsyncData(data);
  }

  Future<MarksData> _update() async {
    var repo = await ref.read(marksRepositoryProvider.future);
    var gb = await ref.read(gbProvider.future);
    var feature = gb.feature("fetch-marks");
    if (feature.on && feature.value) {
      var data = await UpdateMarksUsecase(repo).call();
      return data;
    } else {
      throw FeatureDisabledException("Marks Feature Disabled");
    }
  }
}
