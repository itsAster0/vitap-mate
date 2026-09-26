// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'academic_calendar_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(academicCalendarRepository)
final academicCalendarRepositoryProvider =
    AcademicCalendarRepositoryProvider._();

final class AcademicCalendarRepositoryProvider
    extends
        $FunctionalProvider<
          AsyncValue<AcademicCalendarRepository>,
          AcademicCalendarRepository,
          FutureOr<AcademicCalendarRepository>
        >
    with
        $FutureModifier<AcademicCalendarRepository>,
        $FutureProvider<AcademicCalendarRepository> {
  AcademicCalendarRepositoryProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'academicCalendarRepositoryProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$academicCalendarRepositoryHash();

  @$internal
  @override
  $FutureProviderElement<AcademicCalendarRepository> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<AcademicCalendarRepository> create(Ref ref) {
    return academicCalendarRepository(ref);
  }
}

String _$academicCalendarRepositoryHash() =>
    r'e8f152e166979cb8e3c020cb0fc65095e1164d66';

@ProviderFor(AcademicCalendar)
final academicCalendarProvider = AcademicCalendarProvider._();

final class AcademicCalendarProvider
    extends $AsyncNotifierProvider<AcademicCalendar, AcademicCalendarData> {
  AcademicCalendarProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'academicCalendarProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$academicCalendarHash();

  @$internal
  @override
  AcademicCalendar create() => AcademicCalendar();
}

String _$academicCalendarHash() => r'b7cbd688adbef5597e26c85cc49e679ec1fec5c8';

abstract class _$AcademicCalendar extends $AsyncNotifier<AcademicCalendarData> {
  FutureOr<AcademicCalendarData> build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref =
        this.ref
            as $Ref<AsyncValue<AcademicCalendarData>, AcademicCalendarData>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<
                AsyncValue<AcademicCalendarData>,
                AcademicCalendarData
              >,
              AsyncValue<AcademicCalendarData>,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}

/// The saved calendar in the shape attendance projection uses, or null
/// while it is loading, failed to load, or has no instructional days.

@ProviderFor(semesterCalendar)
final semesterCalendarProvider = SemesterCalendarProvider._();

/// The saved calendar in the shape attendance projection uses, or null
/// while it is loading, failed to load, or has no instructional days.

final class SemesterCalendarProvider
    extends
        $FunctionalProvider<
          SemesterCalendar?,
          SemesterCalendar?,
          SemesterCalendar?
        >
    with $Provider<SemesterCalendar?> {
  /// The saved calendar in the shape attendance projection uses, or null
  /// while it is loading, failed to load, or has no instructional days.
  SemesterCalendarProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'semesterCalendarProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$semesterCalendarHash();

  @$internal
  @override
  $ProviderElement<SemesterCalendar?> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  SemesterCalendar? create(Ref ref) {
    return semesterCalendar(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(SemesterCalendar? value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<SemesterCalendar?>(value),
    );
  }
}

String _$semesterCalendarHash() => r'15664b13698c74413f90e2a4b4b2c255116b2dcd';
