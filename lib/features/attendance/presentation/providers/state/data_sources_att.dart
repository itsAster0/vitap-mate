import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
import 'package:vitapmate/features/attendance/data/datasources/data_source.dart';

part 'data_sources_att.g.dart';

@riverpod
Future<AttendanceDataSource> attendanceDataSource(Ref ref) async {
  return AttendanceDataSource(
    await ref.read(jsonFileStorageProvider.future),
    () => ref.read(vClientProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}
