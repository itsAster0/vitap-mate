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
         isAutoDispose: false,
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

String _$fullAttendanceHash() => r'90f6569e9261f4ca16fcecf0f5b490cc10643315';

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
        isAutoDispose: false,
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

/// The course's history as last saved, or null if it was never fetched.
/// Never goes to VTOP, so the attendance list can read every course's
/// history without a request each.

@ProviderFor(cachedFullAttendance)
final cachedFullAttendanceProvider = CachedFullAttendanceFamily._();

/// The course's history as last saved, or null if it was never fetched.
/// Never goes to VTOP, so the attendance list can read every course's
/// history without a request each.

final class CachedFullAttendanceProvider
    extends
        $FunctionalProvider<
          AsyncValue<FullAttendanceData?>,
          FullAttendanceData?,
          FutureOr<FullAttendanceData?>
        >
    with
        $FutureModifier<FullAttendanceData?>,
        $FutureProvider<FullAttendanceData?> {
  /// The course's history as last saved, or null if it was never fetched.
  /// Never goes to VTOP, so the attendance list can read every course's
  /// history without a request each.
  CachedFullAttendanceProvider._({
    required CachedFullAttendanceFamily super.from,
    required (String, String) super.argument,
  }) : super(
         retry: null,
         name: r'cachedFullAttendanceProvider',
         isAutoDispose: false,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$cachedFullAttendanceHash();

  @override
  String toString() {
    return r'cachedFullAttendanceProvider'
        ''
        '$argument';
  }

  @$internal
  @override
  $FutureProviderElement<FullAttendanceData?> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<FullAttendanceData?> create(Ref ref) {
    final argument = this.argument as (String, String);
    return cachedFullAttendance(ref, argument.$1, argument.$2);
  }

  @override
  bool operator ==(Object other) {
    return other is CachedFullAttendanceProvider && other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$cachedFullAttendanceHash() =>
    r'b142b6d90cc0e131961d3189cb8eccd7ace01ba2';

/// The course's history as last saved, or null if it was never fetched.
/// Never goes to VTOP, so the attendance list can read every course's
/// history without a request each.

final class CachedFullAttendanceFamily extends $Family
    with
        $FunctionalFamilyOverride<
          FutureOr<FullAttendanceData?>,
          (String, String)
        > {
  CachedFullAttendanceFamily._()
    : super(
        retry: null,
        name: r'cachedFullAttendanceProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: false,
      );

  /// The course's history as last saved, or null if it was never fetched.
  /// Never goes to VTOP, so the attendance list can read every course's
  /// history without a request each.

  CachedFullAttendanceProvider call(String courseType, String courseId) =>
      CachedFullAttendanceProvider._(
        argument: (courseType, courseId),
        from: this,
      );

  @override
  String toString() => r'cachedFullAttendanceProvider';
}
