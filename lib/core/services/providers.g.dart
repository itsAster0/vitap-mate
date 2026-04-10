// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'providers.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

String _$appServicesHash() => r'a8a085c5680ff4c684f4b61541fb4896ae816bab';

/// See also [appServices].
@ProviderFor(appServices)
final appServicesProvider = FutureProvider<AppServices>.internal(
  appServices,
  name: r'appServicesProvider',
  debugGetCreateSourceHash:
      const bool.fromEnvironment('dart.vm.product') ? null : _$appServicesHash,
  dependencies: null,
  allTransitiveDependencies: null,
);

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef AppServicesRef = FutureProviderRef<AppServices>;
String _$appLoggerHash() => r'0a6046304652423fcad38434119a19b8e8dc1c5c';

/// See also [appLogger].
@ProviderFor(appLogger)
final appLoggerProvider = FutureProvider<AppLogger>.internal(
  appLogger,
  name: r'appLoggerProvider',
  debugGetCreateSourceHash:
      const bool.fromEnvironment('dart.vm.product') ? null : _$appLoggerHash,
  dependencies: null,
  allTransitiveDependencies: null,
);

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef AppLoggerRef = FutureProviderRef<AppLogger>;
String _$logsHash() => r'c1a33b1cc6c2ac8dd234f5bfeadc7e6a68095c1b';

/// See also [logs].
@ProviderFor(logs)
final logsProvider = StreamProvider<List<LogEntry>>.internal(
  logs,
  name: r'logsProvider',
  debugGetCreateSourceHash:
      const bool.fromEnvironment('dart.vm.product') ? null : _$logsHash,
  dependencies: null,
  allTransitiveDependencies: null,
);

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef LogsRef = StreamProviderRef<List<LogEntry>>;
String _$activeAccountHash() => r'9f1b5492f6a522b0d6c275998517f153e7a70a83';

/// See also [activeAccount].
@ProviderFor(activeAccount)
final activeAccountProvider = FutureProvider<ActiveAccount?>.internal(
  activeAccount,
  name: r'activeAccountProvider',
  debugGetCreateSourceHash:
      const bool.fromEnvironment('dart.vm.product')
          ? null
          : _$activeAccountHash,
  dependencies: null,
  allTransitiveDependencies: null,
);

@Deprecated('Will be removed in 3.0. Use Ref instead')
// ignore: unused_element
typedef ActiveAccountRef = FutureProviderRef<ActiveAccount?>;
// ignore_for_file: type=lint
// ignore_for_file: subtype_of_sealed_class, invalid_use_of_internal_member, invalid_use_of_visible_for_testing_member, deprecated_member_use_from_same_package
