// ignore: depend_on_referenced_packages
import 'package:riverpod/riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/database/app_db_provider.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/features/attendance/data/datasources/local_data_source.dart';
import 'package:vitapmate/features/attendance/data/datasources/remote_data_source.dart';

part 'data_sources_att.g.dart';

@riverpod
Future<AttendanceLocalDataSource> attendanceLocalDataSource(Ref ref) async {
  return AttendanceLocalDataSource(
    await ref.read(appDatabaseProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}

@riverpod
Future<AttendanceRemoteDataSource> attendanceRemoteDataSource(Ref ref) async {
  return AttendanceRemoteDataSource(
    await ref.watch(vClientProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}
