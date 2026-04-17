// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'grade_history_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(GradeHistory)
final gradeHistoryProvider = GradeHistoryProvider._();

final class GradeHistoryProvider
    extends $AsyncNotifierProvider<GradeHistory, GradeHistoryData> {
  GradeHistoryProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'gradeHistoryProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$gradeHistoryHash();

  @$internal
  @override
  GradeHistory create() => GradeHistory();
}

String _$gradeHistoryHash() => r'e0f95d187d65cd5bdf4e84dd3734014349bea10f';

abstract class _$GradeHistory extends $AsyncNotifier<GradeHistoryData> {
  FutureOr<GradeHistoryData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref =
        this.ref as $Ref<AsyncValue<GradeHistoryData>, GradeHistoryData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<GradeHistoryData>, GradeHistoryData>,
              AsyncValue<GradeHistoryData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
