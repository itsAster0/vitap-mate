// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'full_attendance_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(FullAttendance)
final fullAttendanceProvider = FullAttendanceFamily._();

final class FullAttendanceProvider
    extends $AsyncNotifierProvider<FullAttendance, FullAttendanceData> {
  FullAttendanceProvider._({
    required FullAttendanceFamily super.from,
    required (String, String) super.argument,
  }) : super(
         retry: null,
         name: r'fullAttendanceProvider',
         isAutoDispose: true,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$fullAttendanceHash();

  @override
  String toString() {
    return r'fullAttendanceProvider'
        ''
        '$argument';
  }

  @$internal
  @override
  FullAttendance create() => FullAttendance();

  @override
  bool operator ==(Object other) {
    return other is FullAttendanceProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$fullAttendanceHash() => r'6d3a844c231da49b41e674d0cf65115fa749cd68';

final class FullAttendanceFamily extends $Family
    with
        $ClassFamilyOverride<
          FullAttendance,
          AsyncValue<FullAttendanceData>,
          FullAttendanceData,
          FutureOr<FullAttendanceData>,
          (String, String)
        > {
  FullAttendanceFamily._()
    : super(
        retry: null,
        name: r'fullAttendanceProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: true,
      );

  FullAttendanceProvider call(String courseType, String courseId) =>
      FullAttendanceProvider._(argument: (courseType, courseId), from: this);

  @override
  String toString() => r'fullAttendanceProvider';
}

abstract class _$FullAttendance extends $AsyncNotifier<FullAttendanceData> {
  late final _$args = ref.$arg as (String, String);
  String get courseType => _$args.$1;
  String get courseId => _$args.$2;

  FutureOr<FullAttendanceData> build(String courseType, String courseId);
  @$mustCallSuper
  @override
  void runBuild() {
    final ref =
        this.ref as $Ref<AsyncValue<FullAttendanceData>, FullAttendanceData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<FullAttendanceData>, FullAttendanceData>,
              AsyncValue<FullAttendanceData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, () => build(_$args.$1, _$args.$2));
  }
}
