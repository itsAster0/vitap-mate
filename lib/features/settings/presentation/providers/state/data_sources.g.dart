// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'data_sources.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(semidlocalDataSource)
final semidlocalDataSourceProvider = SemidlocalDataSourceProvider._();

final class SemidlocalDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<SemesterIdLocalDataSource>,
          SemesterIdLocalDataSource,
          FutureOr<SemesterIdLocalDataSource>
        >
    with
        $FutureModifier<SemesterIdLocalDataSource>,
        $FutureProvider<SemesterIdLocalDataSource> {
  SemidlocalDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'semidlocalDataSourceProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$semidlocalDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<SemesterIdLocalDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<SemesterIdLocalDataSource> create(Ref ref) {
    return semidlocalDataSource(ref);
  }
}

String _$semidlocalDataSourceHash() =>
    r'982b5d686ed58666cf8b01db8dd2f621b0a38a7d';

@ProviderFor(semidRemoteDataSource)
final semidRemoteDataSourceProvider = SemidRemoteDataSourceProvider._();

final class SemidRemoteDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<SemesterIdRemoteDataSource>,
          SemesterIdRemoteDataSource,
          FutureOr<SemesterIdRemoteDataSource>
        >
    with
        $FutureModifier<SemesterIdRemoteDataSource>,
        $FutureProvider<SemesterIdRemoteDataSource> {
  SemidRemoteDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'semidRemoteDataSourceProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$semidRemoteDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<SemesterIdRemoteDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<SemesterIdRemoteDataSource> create(Ref ref) {
    return semidRemoteDataSource(ref);
  }
}

String _$semidRemoteDataSourceHash() =>
    r'fec311ac1940f078f405ba59ff8080eaf16902f0';
