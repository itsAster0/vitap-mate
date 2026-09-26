// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'timetable_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(Timetable)
final timetableProvider = TimetableProvider._();

final class TimetableProvider
    extends $AsyncNotifierProvider<Timetable, TimetableData> {
  TimetableProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'timetableProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$timetableHash();

  @$internal
  @override
  Timetable create() => Timetable();
}

String _$timetableHash() => r'6804d56f0e4f05115f9c1e7cb2180b74d467476a';

abstract class _$Timetable extends $AsyncNotifier<TimetableData> {
  FutureOr<TimetableData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<TimetableData>, TimetableData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<TimetableData>, TimetableData>,
              AsyncValue<TimetableData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
