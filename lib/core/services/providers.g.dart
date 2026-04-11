// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'providers.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(appServices)
final appServicesProvider = AppServicesProvider._();

final class AppServicesProvider
    extends
        $FunctionalProvider<
          AsyncValue<AppServices>,
          AppServices,
          FutureOr<AppServices>
        >
    with $FutureModifier<AppServices>, $FutureProvider<AppServices> {
  AppServicesProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'appServicesProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$appServicesHash();

  @$internal
  @override
  $FutureProviderElement<AppServices> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<AppServices> create(Ref ref) {
    return appServices(ref);
  }
}

String _$appServicesHash() => r'a8a085c5680ff4c684f4b61541fb4896ae816bab';

@ProviderFor(appLogger)
final appLoggerProvider = AppLoggerProvider._();

final class AppLoggerProvider
    extends
        $FunctionalProvider<
          AsyncValue<AppLogger>,
          AppLogger,
          FutureOr<AppLogger>
        >
    with $FutureModifier<AppLogger>, $FutureProvider<AppLogger> {
  AppLoggerProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'appLoggerProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$appLoggerHash();

  @$internal
  @override
  $FutureProviderElement<AppLogger> $createElement($ProviderPointer pointer) =>
      $FutureProviderElement(pointer);

  @override
  FutureOr<AppLogger> create(Ref ref) {
    return appLogger(ref);
  }
}

String _$appLoggerHash() => r'0a6046304652423fcad38434119a19b8e8dc1c5c';

@ProviderFor(logs)
final logsProvider = LogsProvider._();

final class LogsProvider
    extends
        $FunctionalProvider<
          AsyncValue<List<LogEntry>>,
          List<LogEntry>,
          Stream<List<LogEntry>>
        >
    with $FutureModifier<List<LogEntry>>, $StreamProvider<List<LogEntry>> {
  LogsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'logsProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$logsHash();

  @$internal
  @override
  $StreamProviderElement<List<LogEntry>> $createElement(
    $ProviderPointer pointer,
  ) => $StreamProviderElement(pointer);

  @override
  Stream<List<LogEntry>> create(Ref ref) {
    return logs(ref);
  }
}

String _$logsHash() => r'c1a33b1cc6c2ac8dd234f5bfeadc7e6a68095c1b';

@ProviderFor(activeAccount)
final activeAccountProvider = ActiveAccountProvider._();

final class ActiveAccountProvider
    extends
        $FunctionalProvider<
          AsyncValue<ActiveAccount?>,
          ActiveAccount?,
          FutureOr<ActiveAccount?>
        >
    with $FutureModifier<ActiveAccount?>, $FutureProvider<ActiveAccount?> {
  ActiveAccountProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'activeAccountProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$activeAccountHash();

  @$internal
  @override
  $FutureProviderElement<ActiveAccount?> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<ActiveAccount?> create(Ref ref) {
    return activeAccount(ref);
  }
}

String _$activeAccountHash() => r'9f1b5492f6a522b0d6c275998517f153e7a70a83';
