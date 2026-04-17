// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'semester_id.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(semidRepository)
final semidRepositoryProvider = SemidRepositoryProvider._();

final class SemidRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<SemidRepository>,
          SemidRepository,
          FutureOr<SemidRepository>
        >
    with $FutureModifier<SemidRepository>, $FutureProvider<SemidRepository> {
  SemidRepositoryProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'semidRepositoryProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$semidRepositoryHash();

  @$internal
  @override
  $FutureProviderElement<SemidRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<SemidRepository> create(Ref ref) {
    return semidRepository(ref);
  }
}

String _$semidRepositoryHash() => r'c4d7918822009843b6689cee98077dbb5a093ab4';
