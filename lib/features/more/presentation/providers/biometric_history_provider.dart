import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/vtop_controller.dart';
import 'package:vitapmate/features/more/presentation/providers/state/biometric_history_repository.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

part 'biometric_history_provider.g.dart';

@Riverpod(keepAlive: true)
class BiometricHistory extends _$BiometricHistory {
  @override
  Future<BiometricData> build(String date) async {
    final repository = await ref.watch(
      biometricHistoryRepositoryProvider(date).future,
    );
    return VtopController<BiometricData>(
      ref: ref,
      repository: repository,
      featureName: 'fetch-biometric-history',
    ).load();
  }

  Future<void> refresh() async {
    final repository = await ref.read(
      biometricHistoryRepositoryProvider(date).future,
    );
    final data = await VtopController<BiometricData>(
      ref: ref,
      repository: repository,
      featureName: 'fetch-biometric-history',
    ).refresh();
    state = AsyncData(data);
  }
}
