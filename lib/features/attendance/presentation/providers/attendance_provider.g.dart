// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'attendance_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(Attendance)
final attendanceProvider = AttendanceProvider._();

final class AttendanceProvider
    extends $AsyncNotifierProvider<Attendance, AttendanceData> {
  AttendanceProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'attendanceProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$attendanceHash();

  @$internal
  @override
  Attendance create() => Attendance();
}

String _$attendanceHash() => r'b4c831a0b9fb5ec82666eae6ad5751bdf51865c2';

abstract class _$Attendance extends $AsyncNotifier<AttendanceData> {
  FutureOr<AttendanceData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AsyncValue<AttendanceData>, AttendanceData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AsyncValue<AttendanceData>, AttendanceData>,
              AsyncValue<AttendanceData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
