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
        isAutoDispose: true,
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
    r'3f7500ed77eb89c75454ad2a632bd5b5b93baf3b';
