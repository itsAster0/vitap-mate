// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'settings.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(settings)
final settingsProvider = SettingsProvider._();

final class SettingsProvider
    extends
        $FunctionalProvider<
          AsyncValue<SharedPreferencesWithCache>,
          SharedPreferencesWithCache,
          FutureOr<SharedPreferencesWithCache>
        >
    with
        $FutureModifier<SharedPreferencesWithCache>,
        $FutureProvider<SharedPreferencesWithCache> {
  SettingsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'settingsProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$settingsHash();

  @$internal
  @override
  $FutureProviderElement<SharedPreferencesWithCache> $createElement(
    $ProviderPointer pointer,
  ) => $FutureProviderElement(pointer);

  @override
  FutureOr<SharedPreferencesWithCache> create(Ref ref) {
    return settings(ref);
  }
}

String _$settingsHash() => r'ceb9b7f7845d6820edc184147b546cbe4b9e5fa3';

/// The optional vtop-server. The API key is kept in plain preferences by
/// choice; see rust/ARCHITECTURE.md.

@ProviderFor(vtopServerSettings)
final vtopServerSettingsProvider = VtopServerSettingsProvider._();

/// The optional vtop-server. The API key is kept in plain preferences by
/// choice; see rust/ARCHITECTURE.md.

final class VtopServerSettingsProvider
    extends
        $FunctionalProvider<
          VtopServerSettings,
          VtopServerSettings,
          VtopServerSettings
        >
    with $Provider<VtopServerSettings> {
  /// The optional vtop-server. The API key is kept in plain preferences by
  /// choice; see rust/ARCHITECTURE.md.
  VtopServerSettingsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vtopServerSettingsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vtopServerSettingsHash();

  @$internal
  @override
  $ProviderElement<VtopServerSettings> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  VtopServerSettings create(Ref ref) {
    return vtopServerSettings(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(VtopServerSettings value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<VtopServerSettings>(value),
    );
  }
}

String _$vtopServerSettingsHash() =>
    r'8f0025ef6f3ae780df89b4c11271de4a5282cba4';

@ProviderFor(classesLeftUntil)
final classesLeftUntilProvider = ClassesLeftUntilProvider._();

final class ClassesLeftUntilProvider
    extends
        $FunctionalProvider<
          ClassesLeftUntil,
          ClassesLeftUntil,
          ClassesLeftUntil
        >
    with $Provider<ClassesLeftUntil> {
  ClassesLeftUntilProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'classesLeftUntilProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$classesLeftUntilHash();

  @$internal
  @override
  $ProviderElement<ClassesLeftUntil> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  ClassesLeftUntil create(Ref ref) {
    return classesLeftUntil(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ClassesLeftUntil value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ClassesLeftUntil>(value),
    );
  }
}

String _$classesLeftUntilHash() => r'87980823782c1ba0b332a8cbbd5b428259e4a347';

/// The floating refresh button on the screen edge.

@ProviderFor(refreshButton)
final refreshButtonProvider = RefreshButtonProvider._();

/// The floating refresh button on the screen edge.

final class RefreshButtonProvider extends $FunctionalProvider<bool, bool, bool>
    with $Provider<bool> {
  /// The floating refresh button on the screen edge.
  RefreshButtonProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'refreshButtonProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$refreshButtonHash();

  @$internal
  @override
  $ProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  bool create(Ref ref) {
    return refreshButton(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$refreshButtonHash() => r'0711bfcdcccc930f573c3356d9adf8793e7fbe75';

@ProviderFor(autoRefresh)
final autoRefreshProvider = AutoRefreshProvider._();

final class AutoRefreshProvider extends $FunctionalProvider<bool, bool, bool>
    with $Provider<bool> {
  AutoRefreshProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'autoRefreshProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$autoRefreshHash();

  @$internal
  @override
  $ProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  bool create(Ref ref) {
    return autoRefresh(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$autoRefreshHash() => r'4c2d0a94323a07048e4d82a634ec36e46afd5ad6';

@ProviderFor(emailOtpDeleteAfterReading)
final emailOtpDeleteAfterReadingProvider =
    EmailOtpDeleteAfterReadingProvider._();

final class EmailOtpDeleteAfterReadingProvider
    extends $FunctionalProvider<bool, bool, bool>
    with $Provider<bool> {
  EmailOtpDeleteAfterReadingProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'emailOtpDeleteAfterReadingProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$emailOtpDeleteAfterReadingHash();

  @$internal
  @override
  $ProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  bool create(Ref ref) {
    return emailOtpDeleteAfterReading(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$emailOtpDeleteAfterReadingHash() =>
    r'64a529c0ea1d3c31f67033805e75c5cef5b1a827';

@ProviderFor(vtopSessionReuseTtl)
final vtopSessionReuseTtlProvider = VtopSessionReuseTtlProvider._();

final class VtopSessionReuseTtlProvider
    extends $FunctionalProvider<Duration, Duration, Duration>
    with $Provider<Duration> {
  VtopSessionReuseTtlProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'vtopSessionReuseTtlProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$vtopSessionReuseTtlHash();

  @$internal
  @override
  $ProviderElement<Duration> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  Duration create(Ref ref) {
    return vtopSessionReuseTtl(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(Duration value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<Duration>(value),
    );
  }
}

String _$vtopSessionReuseTtlHash() =>
    r'32275c9b29c9ed3ba43b5a67b7c1e0e14836ed68';

@ProviderFor(studentProjectPinnedIds)
final studentProjectPinnedIdsProvider = StudentProjectPinnedIdsProvider._();

final class StudentProjectPinnedIdsProvider
    extends $FunctionalProvider<Set<int>, Set<int>, Set<int>>
    with $Provider<Set<int>> {
  StudentProjectPinnedIdsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'studentProjectPinnedIdsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$studentProjectPinnedIdsHash();

  @$internal
  @override
  $ProviderElement<Set<int>> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  Set<int> create(Ref ref) {
    return studentProjectPinnedIds(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(Set<int> value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<Set<int>>(value),
    );
  }
}

String _$studentProjectPinnedIdsHash() =>
    r'9bba55d0d44e1aabcebc1bbe66acecd2e7b47de5';

@ProviderFor(StudentProjectsPinnedOnlySession)
final studentProjectsPinnedOnlySessionProvider =
    StudentProjectsPinnedOnlySessionProvider._();

final class StudentProjectsPinnedOnlySessionProvider
    extends $NotifierProvider<StudentProjectsPinnedOnlySession, bool> {
  StudentProjectsPinnedOnlySessionProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'studentProjectsPinnedOnlySessionProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$studentProjectsPinnedOnlySessionHash();

  @$internal
  @override
  StudentProjectsPinnedOnlySession create() =>
      StudentProjectsPinnedOnlySession();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$studentProjectsPinnedOnlySessionHash() =>
    r'ae1c6c87f2c3ab6b64a96f50925000259c08ae6a';

abstract class _$StudentProjectsPinnedOnlySession extends $Notifier<bool> {
  bool build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<bool, bool>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<bool, bool>,
              bool,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}

@ProviderFor(studentProjectsSettingsController)
final studentProjectsSettingsControllerProvider =
    StudentProjectsSettingsControllerProvider._();

final class StudentProjectsSettingsControllerProvider
    extends
        $FunctionalProvider<
          StudentProjectsSettingsController,
          StudentProjectsSettingsController,
          StudentProjectsSettingsController
        >
    with $Provider<StudentProjectsSettingsController> {
  StudentProjectsSettingsControllerProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'studentProjectsSettingsControllerProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() =>
      _$studentProjectsSettingsControllerHash();

  @$internal
  @override
  $ProviderElement<StudentProjectsSettingsController> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  StudentProjectsSettingsController create(Ref ref) {
    return studentProjectsSettingsController(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(StudentProjectsSettingsController value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<StudentProjectsSettingsController>(
        value,
      ),
    );
  }
}

String _$studentProjectsSettingsControllerHash() =>
    r'019e40b28dfa09889d3088a245789e386fde9c49';

@ProviderFor(classReminderSettings)
final classReminderSettingsProvider = ClassReminderSettingsProvider._();

final class ClassReminderSettingsProvider
    extends
        $FunctionalProvider<
          ClassReminderSettings,
          ClassReminderSettings,
          ClassReminderSettings
        >
    with $Provider<ClassReminderSettings> {
  ClassReminderSettingsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'classReminderSettingsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$classReminderSettingsHash();

  @$internal
  @override
  $ProviderElement<ClassReminderSettings> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  ClassReminderSettings create(Ref ref) {
    return classReminderSettings(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ClassReminderSettings value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ClassReminderSettings>(value),
    );
  }
}

String _$classReminderSettingsHash() =>
    r'f9ea74430e3c44d719f806d37acc4adde6dd2f0b';

@ProviderFor(classReminderSettingsController)
final classReminderSettingsControllerProvider =
    ClassReminderSettingsControllerProvider._();

final class ClassReminderSettingsControllerProvider
    extends
        $FunctionalProvider<
          ClassReminderSettingsController,
          ClassReminderSettingsController,
          ClassReminderSettingsController
        >
    with $Provider<ClassReminderSettingsController> {
  ClassReminderSettingsControllerProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'classReminderSettingsControllerProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$classReminderSettingsControllerHash();

  @$internal
  @override
  $ProviderElement<ClassReminderSettingsController> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  ClassReminderSettingsController create(Ref ref) {
    return classReminderSettingsController(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ClassReminderSettingsController value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ClassReminderSettingsController>(
        value,
      ),
    );
  }
}

String _$classReminderSettingsControllerHash() =>
    r'8bfd4bc5e7f0e0ca582931a183103a446e6ff273';

@ProviderFor(examReminderSettings)
final examReminderSettingsProvider = ExamReminderSettingsProvider._();

final class ExamReminderSettingsProvider
    extends
        $FunctionalProvider<
          ExamReminderSettings,
          ExamReminderSettings,
          ExamReminderSettings
        >
    with $Provider<ExamReminderSettings> {
  ExamReminderSettingsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'examReminderSettingsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$examReminderSettingsHash();

  @$internal
  @override
  $ProviderElement<ExamReminderSettings> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  ExamReminderSettings create(Ref ref) {
    return examReminderSettings(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ExamReminderSettings value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ExamReminderSettings>(value),
    );
  }
}

String _$examReminderSettingsHash() =>
    r'd250da3ff304498381ee39343e9ac11baf175d08';

@ProviderFor(examReminderSettingsController)
final examReminderSettingsControllerProvider =
    ExamReminderSettingsControllerProvider._();

final class ExamReminderSettingsControllerProvider
    extends
        $FunctionalProvider<
          ExamReminderSettingsController,
          ExamReminderSettingsController,
          ExamReminderSettingsController
        >
    with $Provider<ExamReminderSettingsController> {
  ExamReminderSettingsControllerProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'examReminderSettingsControllerProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$examReminderSettingsControllerHash();

  @$internal
  @override
  $ProviderElement<ExamReminderSettingsController> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  ExamReminderSettingsController create(Ref ref) {
    return examReminderSettingsController(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ExamReminderSettingsController value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ExamReminderSettingsController>(
        value,
      ),
    );
  }
}

String _$examReminderSettingsControllerHash() =>
    r'307386b85a678b5bdb9faecf2dac3740efc17dcd';

@ProviderFor(changeAlertsSettings)
final changeAlertsSettingsProvider = ChangeAlertsSettingsProvider._();

final class ChangeAlertsSettingsProvider
    extends
        $FunctionalProvider<
          ChangeAlertsSettings,
          ChangeAlertsSettings,
          ChangeAlertsSettings
        >
    with $Provider<ChangeAlertsSettings> {
  ChangeAlertsSettingsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'changeAlertsSettingsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$changeAlertsSettingsHash();

  @$internal
  @override
  $ProviderElement<ChangeAlertsSettings> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  ChangeAlertsSettings create(Ref ref) {
    return changeAlertsSettings(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ChangeAlertsSettings value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ChangeAlertsSettings>(value),
    );
  }
}

String _$changeAlertsSettingsHash() =>
    r'3032c32317f1539c6519671fb187de2fc46a32a8';

@ProviderFor(changeAlertsSettingsController)
final changeAlertsSettingsControllerProvider =
    ChangeAlertsSettingsControllerProvider._();

final class ChangeAlertsSettingsControllerProvider
    extends
        $FunctionalProvider<
          ChangeAlertsSettingsController,
          ChangeAlertsSettingsController,
          ChangeAlertsSettingsController
        >
    with $Provider<ChangeAlertsSettingsController> {
  ChangeAlertsSettingsControllerProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'changeAlertsSettingsControllerProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$changeAlertsSettingsControllerHash();

  @$internal
  @override
  $ProviderElement<ChangeAlertsSettingsController> $createElement(
    $ProviderPointer pointer,
  ) => $ProviderElement(pointer);

  @override
  ChangeAlertsSettingsController create(Ref ref) {
    return changeAlertsSettingsController(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ChangeAlertsSettingsController value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ChangeAlertsSettingsController>(
        value,
      ),
    );
  }
}

String _$changeAlertsSettingsControllerHash() =>
    r'c154099aeb8618a8a36244942950de479a6a7258';
