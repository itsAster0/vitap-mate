// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'data_source_tt.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(timetableLocalDataSource)
final timetableLocalDataSourceProvider = TimetableLocalDataSourceProvider._();

final class TimetableLocalDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<TimetableLocalDataSource>,
          TimetableLocalDataSource,
          FutureOr<TimetableLocalDataSource>
        >
    with
        $FutureModifier<TimetableLocalDataSource>,
        $FutureProvider<TimetableLocalDataSource> {
  TimetableLocalDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'timetableLocalDataSourceProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$timetableLocalDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<TimetableLocalDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<TimetableLocalDataSource> create(Ref ref) {
    return timetableLocalDataSource(ref);
  }
}

String _$timetableLocalDataSourceHash() =>
    r'1518c0c6feff2b62a0d738e3d9a347d113f3b0ea';

@ProviderFor(timetableRemoteDataSource)
final timetableRemoteDataSourceProvider = TimetableRemoteDataSourceProvider._();

final class TimetableRemoteDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<TimetableRemoteDataSource>,
          TimetableRemoteDataSource,
          FutureOr<TimetableRemoteDataSource>
        >
    with
        $FutureModifier<TimetableRemoteDataSource>,
        $FutureProvider<TimetableRemoteDataSource> {
  TimetableRemoteDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'timetableRemoteDataSourceProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$timetableRemoteDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<TimetableRemoteDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<TimetableRemoteDataSource> create(Ref ref) {
    return timetableRemoteDataSource(ref);
  }
}

String _$timetableRemoteDataSourceHash() =>
    r'157f3a0c15ce8f057ff2e7f72288c9dbfa4e437a';
