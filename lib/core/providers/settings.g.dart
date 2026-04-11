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

String _$settingsHash() => r'73cb70c94c63cbe7b07dac26281c422c5e3692eb';

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

@ProviderFor(toggleMergeTT)
final toggleMergeTTProvider = ToggleMergeTTProvider._();

final class ToggleMergeTTProvider
    extends $FunctionalProvider<AsyncValue<void>, void, FutureOr<void>>
    with $FutureModifier<void>, $FutureProvider<void> {
  ToggleMergeTTProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'toggleMergeTTProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$toggleMergeTTHash();

  @$internal
  @override
  $FutureProviderElement<void> $createElement($ProviderPointer pointer) =>
      $FutureProviderElement(pointer);

  @override
  FutureOr<void> create(Ref ref) {
    return toggleMergeTT(ref);
  }
}

String _$toggleMergeTTHash() => r'4089aa249c098148ae172a46690855636c7370b8';

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

@ProviderFor(toggleBTWExams)
final toggleBTWExamsProvider = ToggleBTWExamsProvider._();

final class ToggleBTWExamsProvider
    extends $FunctionalProvider<AsyncValue<void>, void, FutureOr<void>>
    with $FutureModifier<void>, $FutureProvider<void> {
  ToggleBTWExamsProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'toggleBTWExamsProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$toggleBTWExamsHash();

  @$internal
  @override
  $FutureProviderElement<void> $createElement($ProviderPointer pointer) =>
      $FutureProviderElement(pointer);

  @override
  FutureOr<void> create(Ref ref) {
    return toggleBTWExams(ref);
  }
}

String _$toggleBTWExamsHash() => r'd6f29e77007ef71042f30310115986286dd5522c';

@ProviderFor(autoRefreshOnOpen)
final autoRefreshOnOpenProvider = AutoRefreshOnOpenProvider._();

final class AutoRefreshOnOpenProvider
    extends $FunctionalProvider<bool, bool, bool>
    with $Provider<bool> {
  AutoRefreshOnOpenProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'autoRefreshOnOpenProvider',
        isAutoDispose: true,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$autoRefreshOnOpenHash();

  @$internal
  @override
  $ProviderElement<bool> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  bool create(Ref ref) {
    return autoRefreshOnOpen(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(bool value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<bool>(value),
    );
  }
}

String _$autoRefreshOnOpenHash() => r'7764a675809137b4c16effd74988e7fb224fc53a';

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
        isAutoDispose: false,
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
    r'80a6515f1742503a3547b58bb16b946f64f8a32c';

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
        isAutoDispose: false,
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
    r'a907de04556718c380f670b1cf74dd83d773a166';

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
