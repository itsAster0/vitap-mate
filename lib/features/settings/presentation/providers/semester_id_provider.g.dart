// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'semester_id_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(SemesterId)
final semesterIdProvider = SemesterIdProvider._();

final class SemesterIdProvider
    extends $AsyncNotifierProvider<SemesterId, SemesterData> {
  SemesterIdProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'semesterIdProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$semesterIdHash();

  @$internal
  @override
  SemesterId create() => SemesterId();
}

String _$semesterIdHash() => r'f4e02676cbf291ec9547df94f795b2f50eb5bc60';

abstract class _$SemesterId extends $AsyncNotifier<SemesterData> {
  FutureOr<SemesterData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<SemesterData>, SemesterData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<SemesterData>, SemesterData>,
              AsyncValue<SemesterData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
