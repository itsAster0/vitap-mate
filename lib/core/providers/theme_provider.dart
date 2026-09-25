import 'package:flutter/material.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/theme/app_palette.dart';
part 'theme_provider.g.dart';

@Riverpod(keepAlive: true)
class ThemeModeController extends _$ThemeModeController {
  static const String _themeKey = 'theme_mode';

  @override
  ThemeMode build() {
    _loadThemeMode();
    return ThemeMode.system;
  }

  Future<void> _loadThemeMode() async {
    final prefs = await SharedPreferences.getInstance();
    final themeIndex = prefs.getInt(_themeKey) ?? 1;
    state = ThemeMode.values[themeIndex];
  }

  Future<void> setThemeMode(ThemeMode themeMode) async {
    state = themeMode;
    final prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_themeKey, themeMode.index);
  }

  Future<void> toggleTheme() async {
    final newThemeMode = state == ThemeMode.light
        ? ThemeMode.dark
        : ThemeMode.light;
    await setThemeMode(newThemeMode);
  }

  bool get isDarkMode => state == ThemeMode.dark;
  bool get isLightMode => state == ThemeMode.light;
  bool get isSystemMode => state == ThemeMode.system;
}

final themeProvider = themeModeControllerProvider;

@riverpod
FThemeData fTheme(Ref ref) {
  final themeMode = ref.watch(themeProvider);

  final dark = switch (themeMode) {
    ThemeMode.dark => true,
    ThemeMode.light => false,
    ThemeMode.system =>
      WidgetsBinding.instance.platformDispatcher.platformBrightness ==
          Brightness.dark,
  };
  return dark ? appDarkTheme : appLightTheme;
}

// A soft grey page so white cards read as surfaces instead of blending in.
final appLightTheme = _withPalette(
  FTheme.neutral.light.touch,
  AppPalette.light,
  background: const Color(0xFFF5F5F6),
);
final appDarkTheme = _withPalette(FTheme.neutral.dark.touch, AppPalette.dark);

FThemeData _withPalette(
  FThemeData base,
  AppPalette palette, {
  Color? background,
}) => FThemeData(
  colors: base.colors.copyWith(background: background, extensions: [palette]),
  typography: base.typography,
  style: base.style,
  touch: true,
);
