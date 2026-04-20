// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'data_sources_att.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(attendanceDataSource)
final attendanceDataSourceProvider = AttendanceDataSourceProvider._();

final class AttendanceDataSourceProvider
    extends
        $FunctionalProvider<
          AsyncValue<AttendanceDataSource>,
          AttendanceDataSource,
          FutureOr<AttendanceDataSource>
        >
    with
        $FutureModifier<AttendanceDataSource>,
        $FutureProvider<AttendanceDataSource> {
  AttendanceDataSourceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'attendanceDataSourceProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$attendanceDataSourceHash();

  @$internal
  @override
  $FutureProviderElement<AttendanceDataSource> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<AttendanceDataSource> create(Ref ref) {
    return attendanceDataSource(ref);
  }
}

String _$attendanceDataSourceHash() =>
    r'80e04974e670252515b582006e213483be35e104';
