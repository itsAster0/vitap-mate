import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/features/more/domine/usecases/get_grade_history.dart';
import 'package:vitapmate/features/more/domine/usecases/update_grade_history.dart';
import 'package:vitapmate/features/more/presentation/providers/state/exam_schedule.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'grade_history_provider.g.dart';

@riverpod
class GradeHistory extends _$GradeHistory {
  @override
  Future<GradeHistoryData> build() async {
    final repo = await ref.watch(gradeHistoryRepositoryProvider.future);
    var data = await GetGradeHistoryUsecase(repo).call();
    if (data.records.isEmpty) {
      data = (await _update(hasLocalData: false)).$1;
    }
    return data;
  }

  Future<void> refresh() async {
    final repo = await ref.read(gradeHistoryRepositoryProvider.future);
    final cached = await GetGradeHistoryUsecase(repo).call();
    final result = await _update(hasLocalData: cached.records.isNotEmpty);
    final data = result.$1;
    final didFetchRemote = result.$2;
    state = AsyncData(data);
    if (!didFetchRemote) {
      throw FeatureDisabledException("Grade History Feature Disabled");
    }
  }

  Future<(GradeHistoryData, bool)> _update({required bool hasLocalData}) async {
    final repo = await ref.read(gradeHistoryRepositoryProvider.future);
    final gb = await ref.read(gbProvider.future);
    final feature = gb.feature("fetch-grade-history");
    if (feature.on && feature.value) {
      await ref.read(vClientProvider.notifier).tryLogin();
      return (await UpdateGradeHistoryUsecase(repo).call(), true);
    } else {
      if (hasLocalData) {
        return (await GetGradeHistoryUsecase(repo).call(), false);
      }
      throw FeatureDisabledException("Grade History Feature Disabled");
    }
  }
}
