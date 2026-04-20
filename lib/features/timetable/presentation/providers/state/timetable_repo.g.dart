// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'timetable_repo.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(timetableRepository)
final timetableRepositoryProvider = TimetableRepositoryProvider._();

final class TimetableRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<TimetableRepository>,
          TimetableRepository,
          FutureOr<TimetableRepository>
        >
    with
        $FutureModifier<TimetableRepository>,
        $FutureProvider<TimetableRepository> {
  TimetableRepositoryProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'timetableRepositoryProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$timetableRepositoryHash();

  @$internal
  @override
  $FutureProviderElement<TimetableRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<TimetableRepository> create(Ref ref) {
    return timetableRepository(ref);
  }
}

String _$timetableRepositoryHash() =>
    r'547d02e257679cbfffec1cd34eb5d8396a39bb91';
