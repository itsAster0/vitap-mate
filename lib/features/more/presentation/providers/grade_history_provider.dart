import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/services/service_layer.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'grade_history_provider.g.dart';

@riverpod
class GradeHistory extends _$GradeHistory {
  @override
  Future<GradeHistoryData> build() async {
    final services = await ref.watch(appServicesProvider.future);
    var data = await services.vtopDataRepository.loadGradeHistory();
    if (data.records.isEmpty) {
      data = (await _update(hasLocalData: false)).$1;
    }
    return data;
  }

  Future<void> refresh() async {
    final services = await ref.read(appServicesProvider.future);
    final cached = await services.vtopDataRepository.loadGradeHistory();
    final result = await _update(hasLocalData: cached.records.isNotEmpty);
    final data = result.$1;
    final didFetchRemote = result.$2;
    state = AsyncData(data);
    if (!didFetchRemote) {
      throw FeatureDisabledException("Grade History Feature Disabled");
    }
  }

  Future<(GradeHistoryData, bool)> _update({required bool hasLocalData}) async {
    final services = await ref.read(appServicesProvider.future);
    final gb = await ref.read(gbProvider.future);
    final feature = gb.feature("fetch-grade-history");
    if (feature.on && feature.value) {
      return (
        await services.vtopDataRepository.loadGradeHistory(refresh: true),
        true,
      );
    } else {
      if (hasLocalData) {
        return (await services.vtopDataRepository.loadGradeHistory(), false);
      }
      throw FeatureDisabledException("Grade History Feature Disabled");
    }
  }
}
