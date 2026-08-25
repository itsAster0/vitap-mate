// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'attendance_repository.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(attendanceRepository)
final attendanceRepositoryProvider = AttendanceRepositoryProvider._();

final class AttendanceRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<AttendanceRepository>,
          AttendanceRepository,
          FutureOr<AttendanceRepository>
        >
    with
        $FutureModifier<AttendanceRepository>,
        $FutureProvider<AttendanceRepository> {
  AttendanceRepositoryProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'attendanceRepositoryProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$attendanceRepositoryHash();

  @$internal
  @override
  $FutureProviderElement<AttendanceRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<AttendanceRepository> create(Ref ref) {
    return attendanceRepository(ref);
  }
}

String _$attendanceRepositoryHash() =>
    r'2f83321e68da0c1e895334015d73a0c1a471062a';

@ProviderFor(fullAttendanceRepository)
final fullAttendanceRepositoryProvider = FullAttendanceRepositoryFamily._();

final class FullAttendanceRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<FullAttendanceRepository>,
          FullAttendanceRepository,
          FutureOr<FullAttendanceRepository>
        >
    with
        $FutureModifier<FullAttendanceRepository>,
        $FutureProvider<FullAttendanceRepository> {
  FullAttendanceRepositoryProvider._({
    required FullAttendanceRepositoryFamily super.from,
    required (String, String) super.argument,
  }) : super(
         retry: null,
         name: r'fullAttendanceRepositoryProvider',
         isAutoDispose: false,
         dependencies: null,
         $allTransitiveDependencies: null,
       );

  @override
  String debugGetCreateSourceHash() => _$fullAttendanceRepositoryHash();

  @override
  String toString() {
    return r'fullAttendanceRepositoryProvider'
        ''
        '$argument';
  }

  @$internal
  @override
  $FutureProviderElement<FullAttendanceRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<FullAttendanceRepository> create(Ref ref) {
    final argument = this.argument as (String, String);
    return fullAttendanceRepository(ref, argument.$1, argument.$2);
  }

  @override
  bool operator ==(Object other) {
    return other is FullAttendanceRepositoryProvider &&
        other.argument == argument;
  }

  @override
  int get hashCode {
    return argument.hashCode;
  }
}

String _$fullAttendanceRepositoryHash() =>
    r'0bc9be35350fe3d60743ecfa17fbcbdfbd0d5d78';

final class FullAttendanceRepositoryFamily extends $Family
    with
        $FunctionalFamilyOverride<
          FutureOr<FullAttendanceRepository>,
          (String, String)
        > {
  FullAttendanceRepositoryFamily._()
    : super(
        retry: null,
        name: r'fullAttendanceRepositoryProvider',
        dependencies: null,
        $allTransitiveDependencies: null,
        isAutoDispose: false,
      );

  FullAttendanceRepositoryProvider call(String courseType, String courseId) =>
      FullAttendanceRepositoryProvider._(
        argument: (courseType, courseId),
        from: this,
      );

  @override
  String toString() => r'fullAttendanceRepositoryProvider';
}
