import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/features/more/data/repositories/biometric_history_repo.dart';
import 'package:vitapmate/features/more/presentation/providers/state/data_source.dart';

part 'biometric_history_repository.g.dart';

@Riverpod(keepAlive: true)
Future<BiometricHistoryRepository> biometricHistoryRepository(
  Ref ref,
  String date,
) async {
  return BiometricHistoryRepository(
    date: date,
    dataSource: await ref.watch(biometricHistoryDataSourceProvider.future),
  );
}
