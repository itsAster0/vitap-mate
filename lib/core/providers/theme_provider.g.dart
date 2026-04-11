// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'theme_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(ThemeController)
final themeControllerProvider = ThemeControllerProvider._();

final class ThemeControllerProvider
    extends $NotifierProvider<ThemeController, ThemeMode> {
  ThemeControllerProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'themeControllerProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$themeControllerHash();

  @$internal
  @override
  ThemeController create() => ThemeController();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ThemeMode value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ThemeMode>(value),
    );
  }
}

String _$themeControllerHash() => r'fd201be2d07cbe4a55b065b27ef70571a4f2e6f9';

abstract class _$ThemeController extends $Notifier<ThemeMode> {
  ThemeMode build();
  @$mustCallSuper
  @override
  void runBuild() {
    final ref = this.ref as $Ref<ThemeMode, ThemeMode>;
    final element =
        ref.element
            as $ClassProviderElement<
              AnyNotifier<ThemeMode, ThemeMode>,
              ThemeMode,
              Object?,
              Object?
            >;
    element.handleCreate(ref, build);
  }
}

@ProviderFor(fTheme)
final fThemeProvider = FThemeProvider._();

final class FThemeProvider
    extends $FunctionalProvider<FThemeData, FThemeData, FThemeData>
    with $Provider<FThemeData> {
  FThemeProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'fThemeProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$fThemeHash();

  @$internal
  @override
  $ProviderElement<FThemeData> $createElement($ProviderPointer pointer) =>
      $ProviderElement(pointer);

  @override
  FThemeData create(Ref ref) {
    return fTheme(ref);
  }

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(FThemeData value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<FThemeData>(value),
    );
  }
}

String _$fThemeHash() => r'2d8b5267921a592bb4374c0593376dcdd7d783ef';
