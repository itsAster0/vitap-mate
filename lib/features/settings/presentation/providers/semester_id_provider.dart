import 'dart:developer';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/features/settings/domine/repositories/semids_repo.dart';
import 'package:vitapmate/features/settings/domine/usecases/get_semester_id.dart';
import 'package:vitapmate/features/settings/domine/usecases/update_semester_id.dart';
import 'package:vitapmate/features/settings/presentation/providers/state/semester_id.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
part 'semester_id_provider.g.dart';

@riverpod
class SemesterId extends _$SemesterId {
  @override
  Future<SemesterData> build() async {
    var semidRepository = await ref.read(semidRepositoryProvider.future);
    var data = await GetSemidsUsecase(semidRepository).call();
    if (data.semesters.isEmpty) {
      data = await UpdateSemidsUsecsae(semidRepository).call();
    } else {
      Future.microtask(() async {
        try {
          await updatesemids();
        } catch (e) {
          log("$e", level: 900);
        }
      });
    }

    return data;
  }

  Future<void> updatesemids() async {
    await ref.read(vClientProvider.notifier).tryLogin();
    // var user = await ref.read(vtopUserProvider.future);
    SemidRepository repo = await ref.read(semidRepositoryProvider.future);
    var data = await UpdateSemidsUsecsae(repo).call();
    state = AsyncData(data);
  }
}
