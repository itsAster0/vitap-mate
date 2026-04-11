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
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$examScheduleHash();

  @$internal
  @override
  ExamSchedule create() => ExamSchedule();
}

String _$examScheduleHash() => r'6f06b67f3afb1a50504e1e9e0a14ef09793a05cb';

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
