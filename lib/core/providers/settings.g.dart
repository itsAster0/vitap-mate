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
        isAutoDispose: true,
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

String _$settingsHash() => r'8da12a0fb629d512f79199851da92eb2463633ba';

@ProviderFor(mergeTT)
final mergeTTProvider = MergeTTProvider._();

final class MergeTTProvider extends $FunctionalProvider<bool, bool, bool>
    with $Provider<bool> {
  MergeTTProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'mergeTTProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$mergeTTHash();

  @$internal
  @override
  $ProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  bool create(Ref ref) {
    return mergeTT(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$mergeTTHash() => r'3511f9380bc7ee242b375bb7efa0594a4aa0187c';

@ProviderFor(btwExams)
final btwExamsProvider = BtwExamsProvider._();

final class BtwExamsProvider extends $FunctionalProvider<bool, bool, bool>
    with $Provider<bool> {
  BtwExamsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'btwExamsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$btwExamsHash();

  @$internal
  @override
  $ProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  bool create(Ref ref) {
    return btwExams(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$btwExamsHash() => r'77cc43c21a5b8dbd6ef2e949431fa4abf5d24a1b';

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

String _$autoRefreshHash() => r'e5ae73c61bbae9b20882fda9e592bd0c6ec00d4f';

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
        isAutoDispose: true,
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
    r'725ff06cf1a7669349581d7f30979b934f19806a';

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
        isAutoDispose: true,
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
    r'3248a7f45836dbb0a55b1125234d569e107d7a1e';

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
        isAutoDispose: true,
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
    r'2df8f91ec2e0cc1b1a8d32fd66d66e70705c672b';
