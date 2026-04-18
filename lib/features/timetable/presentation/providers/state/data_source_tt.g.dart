// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'data_source_tt.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(timetableDataSource)
final timetableDataSourceProvider = TimetableDataSourceProvider._();

final class TimetableDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<TimetableDataSource>,
          TimetableDataSource,
          FutureOr<TimetableDataSource>
        >
    with
        $FutureModifier<TimetableDataSource>,
        $FutureProvider<TimetableDataSource> {
  TimetableDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'timetableDataSourceProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$timetableDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<TimetableDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<TimetableDataSource> create(Ref ref) {
    return timetableDataSource(ref);
  }
}

String _$timetableDataSourceHash() =>
    r'c3c6062c80ebf1009bbebe54bae65da0704eac96';
