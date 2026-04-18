import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
import 'package:vitapmate/features/more/data/datasources/data_source.dart';

part 'data_source.g.dart';

@riverpod
Future<ExamScheduleDataSource> examScheduleDataSource(Ref ref) async {
  return ExamScheduleDataSource(
    await ref.read(jsonFileStorageProvider.future),
    await ref.read(vClientProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}

@riverpod
Future<MarksDataSource> marksDataSource(Ref ref) async {
  return MarksDataSource(
    await ref.read(jsonFileStorageProvider.future),
    await ref.watch(vClientProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}

@riverpod
Future<GradesDataSource> gradesDataSource(Ref ref) async {
  return GradesDataSource(
    await ref.read(jsonFileStorageProvider.future),
    await ref.watch(vClientProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}

@riverpod
Future<GradeHistoryDataSource> gradeHistoryDataSource(Ref ref) async {
  return GradeHistoryDataSource(
    await ref.read(jsonFileStorageProvider.future),
    await ref.watch(vClientProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}
