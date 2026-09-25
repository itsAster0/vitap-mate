import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend_provider.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
import 'package:vitapmate/features/timetable/data/datasources/data_source.dart';

part 'data_source_tt.g.dart';

@Riverpod(keepAlive: true)
Future<TimetableDataSource> timetableDataSource(Ref ref) async {
  return TimetableDataSource(
    await ref.read(jsonFileStorageProvider.future),
    () => ref.read(vtopBackendProvider),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}
