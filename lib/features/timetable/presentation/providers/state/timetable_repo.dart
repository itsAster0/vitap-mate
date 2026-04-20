import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/features/timetable/data/repositories/timetable_repository_impl.dart';
import 'package:vitapmate/features/timetable/presentation/providers/state/data_source_tt.dart';

part 'timetable_repo.g.dart';

@Riverpod(keepAlive: true)
Future<TimetableRepository> timetableRepository(Ref ref) async {
  return TimetableRepository(
    semid: await ref.watch(vtopUserProvider.selectAsync((val) => val.semid!)),
    dataSource: await ref.watch(timetableDataSourceProvider.future),
  );
}
