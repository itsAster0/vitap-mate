import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/database/app_db_provider.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/features/more/data/datasources/local_data_sources.dart';
import 'package:vitapmate/features/more/data/datasources/remote_data_sources.dart';

part 'data_source.g.dart';

@riverpod
Future<ExamScheduleRemoteDataSource> examScheduleRemoteDataSource(
  Ref ref,
) async {
  return ExamScheduleRemoteDataSource(
    await ref.read(vClientProvider.future),

    ref.read(globalAsyncQueueProvider.notifier),
  );
}

@riverpod
Future<ExamScheduleLocalDataSource> examScheduleLocalDataSource(Ref ref) async {
  return ExamScheduleLocalDataSource(
    await ref.watch(appDatabaseProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}

@riverpod
Future<MarksLocalDataSource> marksLocalDataSource(Ref ref) async {
  return MarksLocalDataSource(
    await ref.read(appDatabaseProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}

@riverpod
Future<MarksRemoteDataSource> marksRemoteDataSource(Ref ref) async {
  return MarksRemoteDataSource(
    await ref.watch(vClientProvider.future),
    ref.read(globalAsyncQueueProvider.notifier),
  );
}
