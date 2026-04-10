import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/services/service_layer.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'semester_id_provider.g.dart';

@riverpod
class SemesterId extends _$SemesterId {
  @override
  Future<SemesterData> build() async {
    final services = await ref.watch(appServicesProvider.future);
    var data = await services.vtopDataRepository.loadSemesters();
    if (data.semesters.isEmpty) {
      data = await services.vtopDataRepository.loadSemesters(refresh: true);
    }

    return data;
  }

  Future<void> updatesemids() async {
    final services = await ref.read(appServicesProvider.future);
    var data = await services.vtopDataRepository.loadSemesters(refresh: true);
    state = AsyncData(data);
  }
}
