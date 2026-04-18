import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/utils/cached_repository.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';

class VtopHooks<T> {
  final Future<void> Function()? onStart;
  final Future<void> Function(T data, {required bool fromCache})? onSuccess;
  final Future<void> Function(Object error, StackTrace st)? onError;
  final Future<void> Function()? onComplete;

  const VtopHooks({
    this.onStart,
    this.onSuccess,
    this.onError,
    this.onComplete,
  });
}

class VtopController<T> {
  final Ref ref;
  final CachedRepository<T> repository;
  final String featureName;
  final VtopHooks<T> hooks;

  const VtopController({
    required this.ref,
    required this.repository,
    required this.featureName,
    this.hooks = const VtopHooks(),
  });

  Future<T> load() async {
    return _withHooks(() async {
      AppLogger.instance.info('client.$featureName', 'load called');
      final cached = await repository.loadCache();
      if (cached != null) {
        AppLogger.instance.info('client.$featureName', 'served from cache');
        await hooks.onSuccess?.call(cached, fromCache: true);
        return cached;
      }
      AppLogger.instance.info('client.$featureName', 'cache miss, refreshing');
      return _refreshCore();
    });
  }

  Future<T> refresh() async {
    AppLogger.instance.info('client.$featureName', 'manual refresh requested');
    return _withHooks(_refreshCore);
  }

  Future<T> _withHooks(Future<T> Function() action) async {
    await hooks.onStart?.call();
    try {
      return await action();
    } catch (e, st) {
      await hooks.onError?.call(e, st);
      rethrow;
    } finally {
      await hooks.onComplete?.call();
    }
  }

  Future<T> _refreshCore() async {
    final featureFlags = await ref.read(featureFlagsControllerProvider.future);
    if (!await featureFlags.isEnabled(featureName)) {
      final cached = await repository.loadCache();
      if (cached != null) {
        await hooks.onSuccess?.call(cached, fromCache: true);
        return cached;
      }
      throw FeatureDisabledException("$featureName Feature Disabled");
    }

    await ref.read(vClientProvider.notifier).ensureLogin();
    final data = await repository.refresh();
    AppLogger.instance.info('client.$featureName', 'refresh complete');
    await hooks.onSuccess?.call(data, fromCache: false);
    return data;
  }
}
