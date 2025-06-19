// ignore: depend_on_referenced_packages
import 'package:riverpod/riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/features/timetable/data/repositories/timetable_repository_impl.dart';
import 'package:vitapmate/features/timetable/domine/repositories/timetable_repository.dart';
import 'package:vitapmate/features/timetable/presentation/providers/state/data_source_tt.dart';
part 'timetable_repo.g.dart';

@riverpod
Future<TimetableRepository> timetableRepository(Ref ref) async {
  return TimetableRepositoryImpl(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    remoteDataSource: await ref.watch(timetableRemoteDataSourceProvider.future),
    localDataSource: await ref.watch(timetableLocalDataSourceProvider.future),
  );
}
