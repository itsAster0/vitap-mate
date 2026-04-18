import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/features/settings/data/repositories/semid_repository_impl.dart';
import 'package:vitapmate/features/settings/presentation/providers/state/data_sources.dart';

part 'semester_id.g.dart';

@Riverpod(keepAlive: true)
Future<SemidRepository> semidRepository(Ref ref) async {
  return SemidRepository(await ref.watch(semidDataSourceProvider.future));
}
