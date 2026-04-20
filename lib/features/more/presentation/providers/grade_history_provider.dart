import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/more/presentation/providers/state/exam_schedule.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'grade_history_provider.g.dart';

@Riverpod(keepAlive: true)
class GradeHistory extends _$GradeHistory {
  Future<GradeHistoryData> _runLoad() async {
    final repo = await ref.watch(gradeHistoryRepositoryProvider.future);
    final controller = VtopController<GradeHistoryData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-grade-history",
    );
    return controller.load();
  }

  @override
  Future<GradeHistoryData> build() async {
    return _runLoad();
  }

  Future<void> refresh() async {
    final repo = await ref.read(gradeHistoryRepositoryProvider.future);
    final controller = VtopController<GradeHistoryData>(
      ref: ref,
      repository: repo,
      featureName: "fetch-grade-history",
    );
    final gradeHistory = await controller.refresh();
    state = AsyncData(gradeHistory);
  }
}
