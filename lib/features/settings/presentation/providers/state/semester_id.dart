import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:vitapmate/features/settings/data/repositories/semid_repository_impl.dart';
import 'package:vitapmate/features/settings/domine/repositories/semids_repo.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/features/settings/presentation/providers/state/data_sources.dart';
part 'semester_id.g.dart';

@riverpod
Future<SemidRepository> semidRepository(Ref ref) async {
  var local = await ref.watch(semidlocalDataSourceProvider.future);
  var remote = await ref.watch(semidRemoteDataSourceProvider.future);
  return SemidRepositoryImpl(local, remote);
}
