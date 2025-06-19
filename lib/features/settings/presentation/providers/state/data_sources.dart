import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/database/app_db_provider.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/features/settings/data/datasources/local_data_source.dart';
import 'package:vitapmate/features/settings/data/datasources/remote_data_source.dart';

part 'data_sources.g.dart';

@riverpod
Future<SemesterIdLocalDataSource> semidlocalDataSource(Ref ref) async {
  var appDatabase = await ref.read(appDatabaseProvider.future);
  return SemesterIdLocalDataSource(
    ref.read(globalAsyncQueueProvider.notifier),
    appDatabase,
  );
}

@riverpod
Future<SemesterIdRemoteDataSource> semidRemoteDataSource(Ref ref) async {
  var client = await ref.watch(vClientProvider.future);
  return SemesterIdRemoteDataSource(
    ref.read(globalAsyncQueueProvider.notifier),
    client,
  );
}
