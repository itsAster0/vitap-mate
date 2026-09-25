// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'data_sources.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(semidDataSource)
final semidDataSourceProvider = SemidDataSourceProvider._();

final class SemidDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<SemesterIdDataSource>,
          SemesterIdDataSource,
          FutureOr<SemesterIdDataSource>
        >
    with
        $FutureModifier<SemesterIdDataSource>,
        $FutureProvider<SemesterIdDataSource> {
  SemidDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'semidDataSourceProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$semidDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<SemesterIdDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<SemesterIdDataSource> create(Ref ref) {
    return semidDataSource(ref);
  }
}

String _$semidDataSourceHash() => r'6402317cc47365f6bcb467c33c67f4234b2fb6f8';
