import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/models/account.dart';
import 'package:vitapmate/core/models/log_entry.dart';
import 'package:vitapmate/core/services/app_logger.dart';
import 'package:vitapmate/core/services/app_services.dart';

part 'providers.g.dart';

@Riverpod(keepAlive: true)
Future<AppServices> appServices(Ref ref) async {
  return AppServices.create(queue: ref.read(globalAsyncQueueProvider.notifier));
}

@Riverpod(keepAlive: true)
Future<AppLogger> appLogger(Ref ref) async {
  return (await ref.watch(appServicesProvider.future)).logger;
}

@Riverpod(keepAlive: true)
Stream<List<LogEntry>> logs(Ref ref) async* {
  final logger = await ref.watch(appLoggerProvider.future);
  yield logger.entries;
  yield* logger.stream;
}

@Riverpod(keepAlive: true)
Future<ActiveAccount?> activeAccount(Ref ref) async {
  return (await ref.watch(
    appServicesProvider.future,
  )).authRepository.loadActiveAccount();
}
