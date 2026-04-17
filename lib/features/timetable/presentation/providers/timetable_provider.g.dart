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
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$timetableHash();

  @$internal
  @override
  Timetable create() => Timetable();
}

String _$timetableHash() => r'64e32cb0998c38e25998bbcc500e3642730d23d0';

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
