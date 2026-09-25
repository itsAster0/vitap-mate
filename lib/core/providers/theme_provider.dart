import 'dart:math' as math;

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
  typography: _tuned(base.typography),
  style: base.style,
  touch: true,
);

FTypography _tuned(FTypography typography) => typography.copyWith(
  display: _tracked(typography.display),
  body: _tracked(typography.body),
);

/// Inter is drawn for small text; at larger sizes it reads loose unless the
/// tracking tightens. This is Inter's own "dynamic metrics" curve, so headings
/// sit tight and captions keep a little air.
FTypeface _tracked(FTypeface face) {
  TextStyle t(TextStyle style) {
    final size = style.fontSize ?? 14;
    final em = -0.0223 + 0.185 * math.exp(-0.1745 * size);
    return style.copyWith(letterSpacing: em * size);
  }

  return face.copyWith(
    xs3: t(face.xs3),
    xs2: t(face.xs2),
    xs: t(face.xs),
    sm: t(face.sm),
    md: t(face.md),
    lg: t(face.lg),
    xl: t(face.xl),
    xl2: t(face.xl2),
    xl3: t(face.xl3),
    xl4: t(face.xl4),
    xl5: t(face.xl5),
    xl6: t(face.xl6),
    xl7: t(face.xl7),
    xl8: t(face.xl8),
  );
}
