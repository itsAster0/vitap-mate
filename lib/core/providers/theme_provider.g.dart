// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'theme_provider.dart';

// **************************************************************************
// RiverpodGenerator
// **************************************************************************

// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint, type=warning

@ProviderFor(ThemeModeController)
final themeModeControllerProvider = ThemeModeControllerProvider._();

final class ThemeModeControllerProvider
    extends $NotifierProvider<ThemeModeController, ThemeMode> {
  ThemeModeControllerProvider._()
    : super(
        from: null,
        argument: null,
        retry: null,
        name: r'themeModeControllerProvider',
        isAutoDispose: false,
        dependencies: null,
        $allTransitiveDependencies: null,
      );

  @override
  String debugGetCreateSourceHash() => _$themeModeControllerHash();

  @$internal
  @override
  ThemeModeController create() => ThemeModeController();

  /// {@macro riverpod.override_with_value}
  Override overrideWithValue(ThemeMode value) {
    return $ProviderOverride(
      origin: this,
      providerOverride: $SyncValueProvider<ThemeMode>(value),
    );
  }
}

String _$themeModeControllerHash() =>
    r'bdd1b6d3a80161010d0f9b5a4b7e1901fbaf1401';

abstract class _$ThemeModeController extends $Notifier<ThemeMode> {
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
        isAutoDispose: true,
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

String _$fThemeHash() => r'ef36abc8d4f6ec90a323eaed0801dd04926b7824';
