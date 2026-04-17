// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'data_sources_att.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(attendanceLocalDataSource)
final attendanceLocalDataSourceProvider = AttendanceLocalDataSourceProvider._();

final class AttendanceLocalDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<AttendanceLocalDataSource>,
          AttendanceLocalDataSource,
          FutureOr<AttendanceLocalDataSource>
        >
    with
        $FutureModifier<AttendanceLocalDataSource>,
        $FutureProvider<AttendanceLocalDataSource> {
  AttendanceLocalDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'attendanceLocalDataSourceProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$attendanceLocalDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<AttendanceLocalDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<AttendanceLocalDataSource> create(Ref ref) {
    return attendanceLocalDataSource(ref);
  }
}

String _$attendanceLocalDataSourceHash() =>
    r'026993068306340d63b9357612b14da1da72e21f';

@ProviderFor(attendanceRemoteDataSource)
final attendanceRemoteDataSourceProvider =
    AttendanceRemoteDataSourceProvider._();

final class AttendanceRemoteDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<AttendanceRemoteDataSource>,
          AttendanceRemoteDataSource,
          FutureOr<AttendanceRemoteDataSource>
        >
    with
        $FutureModifier<AttendanceRemoteDataSource>,
        $FutureProvider<AttendanceRemoteDataSource> {
  AttendanceRemoteDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'attendanceRemoteDataSourceProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$attendanceRemoteDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<AttendanceRemoteDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<AttendanceRemoteDataSource> create(Ref ref) {
    return attendanceRemoteDataSource(ref);
  }
}

String _$attendanceRemoteDataSourceHash() =>
    r'5c35b6c8311aa2da2c4598c9b7509f32c93dcde5';
