// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'exam_schedule.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(ExamSchedule)
final examScheduleProvider = ExamScheduleProvider._();

final class ExamScheduleProvider
    extends $AsyncNotifierProvider<ExamSchedule, ExamScheduleData> {
  ExamScheduleProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'examScheduleProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$examScheduleHash();

  @$internal
  @override
  ExamSchedule create() => ExamSchedule();
}

String _$examScheduleHash() => r'3a6654fc682ff7c80553ca8e40d59f03e9f03823';

abstract class _$ExamSchedule extends $AsyncNotifier<ExamScheduleData> {
  FutureOr<ExamScheduleData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref =
        this.ref as $Ref<AsyncValue<ExamScheduleData>, ExamScheduleData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<ExamScheduleData>, ExamScheduleData>,
              AsyncValue<ExamScheduleData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}

/// The exam schedule as last saved, or null if it was never fetched. Never
/// goes to VTOP, for screens that only show it in passing.

@ProviderFor(cachedExamSchedule)
final cachedExamScheduleProvider = CachedExamScheduleProvider._();

/// The exam schedule as last saved, or null if it was never fetched. Never
/// goes to VTOP, for screens that only show it in passing.

final class CachedExamScheduleProvider
    extends
        $FunctionalProvider<
          AsyncValue<ExamScheduleData?>,
          ExamScheduleData?,
          FutureOr<ExamScheduleData?>
        >
    with
        $FutureModifier<ExamScheduleData?>,
        $FutureProvider<ExamScheduleData?> {
  /// The exam schedule as last saved, or null if it was never fetched. Never
  /// goes to VTOP, for screens that only show it in passing.
  CachedExamScheduleProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'cachedExamScheduleProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$cachedExamScheduleHash();

  @$internal
  @override
  $FutureProviderElement<ExamScheduleData?> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<ExamScheduleData?> create(Ref ref) {
    return cachedExamSchedule(ref);
  }
}

String _$cachedExamScheduleHash() =>
    r'a813401d7b8323977d610856851d6b5fa90a5394';
