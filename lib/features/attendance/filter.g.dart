// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'filter.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(AttendanceFilterNotifier)
final attendanceFilterProvider = AttendanceFilterNotifierProvider._();

final class AttendanceFilterNotifierProvider
    extends $NotifierProvider<AttendanceFilterNotifier, AttendanceFilter> {
  AttendanceFilterNotifierProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'attendanceFilterProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$attendanceFilterNotifierHash();

  @$internal
  @override
  AttendanceFilterNotifier create() => AttendanceFilterNotifier();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(AttendanceFilter value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<AttendanceFilter>(value),
    );
  }
}

String _$attendanceFilterNotifierHash() =>
    r'62357e10e8cf054f89c204590e8b2f40624af89c';

abstract class _$AttendanceFilterNotifier extends $Notifier<AttendanceFilter> {
  AttendanceFilter build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<AttendanceFilter, AttendanceFilter>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<AttendanceFilter, AttendanceFilter>,
              AttendanceFilter,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}
