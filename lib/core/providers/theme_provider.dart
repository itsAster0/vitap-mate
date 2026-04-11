// ignore_for_file: avoid_public_notifier_properties
import 'package:flutter/material.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:forui/forui.dart';

part 'theme_provider.g.dart';

@Riverpod(keepAlive: true)
class ThemeController extends _$ThemeController {
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
    final newThemeMode =
        state == ThemeMode.light ? ThemeMode.dark : ThemeMode.light;
    await setThemeMode(newThemeMode);
  }

  bool get isDarkMode => state == ThemeMode.dark;
  bool get isLightMode => state == ThemeMode.light;
  bool get isSystemMode => state == ThemeMode.system;
}

@Riverpod(keepAlive: true)
FThemeData fTheme(Ref ref) {
  final themeMode = ref.watch(themeControllerProvider);

  switch (themeMode) {
    case ThemeMode.dark:
      return FThemes.zinc.dark.touch;
    case ThemeMode.light:
      return FThemes.zinc.light.touch;
    case ThemeMode.system:
      final brightness =
          WidgetsBinding.instance.platformDispatcher.platformBrightness;
      return brightness == Brightness.dark
          ? FThemes.zinc.dark.touch
          : FThemes.zinc.light.touch;
  }
}
