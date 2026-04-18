import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/features/settings/presentation/providers/state/semester_id.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'semester_id_provider.g.dart';

@Riverpod(keepAlive: true)
class SemesterId extends _$SemesterId {
  @override
  Future<SemesterData> build() async {
    var semidRepository = await ref.read(semidRepositoryProvider.future);
    var data = await semidRepository.getSemidsFromStorage();
    if (data.semesters.isEmpty) {
      data = await semidRepository.updateSemids();
    }

    return data;
  }

  Future<void> updatesemids() async {
    await ref.read(vClientProvider.notifier).ensureLogin();
    final repo = await ref.read(semidRepositoryProvider.future);
    final data = await repo.updateSemids();
    state = AsyncData(data);
  }
}
