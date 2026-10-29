import 'dart:async';

import 'package:connectivity_plus/connectivity_plus.dart';
import 'package:flutter/widgets.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/observability/log.dart';

/// Logs every provider that fails, by name (never its value).
final class LogProviderFailures extends ProviderObserver {
  const LogProviderFailures();

  @override
  void providerDidFail(
    ProviderObserverContext context,
    Object error,
    StackTrace stackTrace,
  ) {
    final provider = context.provider;
    final name = provider.name ?? '${provider.runtimeType}';
    final argument = provider.argument == null ? '' : '(${provider.argument})';
    Log.warn('provider', '$name$argument failed: $error');
  }
}

/// Logs each screen [router] shows, the app going to and from the
/// background, and the network changing (wifi, mobile, none), until the
/// returned function is called.
VoidCallback logAppEvents(GoRouter router) {
  String? shown;
  void screen() {
    final location = router.routerDelegate.currentConfiguration.uri.path;
    if (location == shown) return;
    shown = location;
    Log.info('screen', location);
  }

  router.routerDelegate.addListener(screen);
  final lifecycle = AppLifecycleListener(
    onStateChange: (state) => Log.info('app', state.name),
  );
  final network = Connectivity().onConnectivityChanged.listen(
    (kinds) => Log.info('network', kinds.map((kind) => kind.name).join(', ')),
  );
  return () {
    router.routerDelegate.removeListener(screen);
    lifecycle.dispose();
    unawaited(network.cancel());
  };
}
