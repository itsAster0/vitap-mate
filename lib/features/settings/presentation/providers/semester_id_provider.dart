import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/settings/presentation/providers/state/semester_id.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'semester_id_provider.g.dart';

@Riverpod(keepAlive: true)
class SemesterId extends _$SemesterId {
  @override
  Future<SemesterData> build() async {
    final repository = await ref.watch(semidRepositoryProvider.future);
    return VtopController<SemesterData>(
      ref: ref,
      repository: repository,
      featureName: 'fetch-semesters',
    ).load();
  }

  Future<void> updatesemids() async {
    final repository = await ref.read(semidRepositoryProvider.future);
    final data = await VtopController<SemesterData>(
      ref: ref,
      repository: repository,
      featureName: 'fetch-semesters',
    ).refresh();
    state = AsyncData(data);
  }
}
