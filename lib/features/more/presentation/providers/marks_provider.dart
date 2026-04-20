import 'dart:developer';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/more/presentation/providers/state/exam_schedule.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'marks_provider.g.dart';

@Riverpod(keepAlive: true)
class Marks extends _$Marks {
  Future<MarksData> _runLoad() async {
    final repo = await ref.watch(marksRepositoryProvider.future);
    final controller = VtopController<MarksData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-marks",
    );
    return controller.load();
  }

  @override
  Future<MarksData> build() async {
    final data = await _runLoad();
    log("marks build done");
    return data;
  }

  Future<void> updatemarks() async {
    final repo = await ref.read(marksRepositoryProvider.future);
    final controller = VtopController<MarksData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-marks",
    );
    final marks = await controller.refresh();
    state = AsyncData(marks);
  }
}
