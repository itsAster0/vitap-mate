import 'package:flutter/material.dart' show ThemeExtension;
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';

/// A colour role: a solid [base] for icons, bars and strokes, a quiet [subtle]
/// fill for badges and highlighted surfaces, and [onSubtle] for text placed on
/// that fill. Every role is tuned separately for light and dark themes so
/// contrast holds in both.
@immutable
class Tone {
  const Tone({
    required this.base,
    required this.subtle,
    required this.onSubtle,
  });

  final Color base;
  final Color subtle;
  final Color onSubtle;

  static Tone lerp(Tone a, Tone b, double t) => Tone(
    base: Color.lerp(a.base, b.base, t)!,
    subtle: Color.lerp(a.subtle, b.subtle, t)!,
    onSubtle: Color.lerp(a.onSubtle, b.onSubtle, t)!,
  );
}

/// App-specific colours layered on forui's neutral theme.
///
/// The neutral base (black/white, greys) carries structure. Colour is reserved
/// for meaning: [accent] marks what is live or selected, and the status tones
/// describe state (good / at risk / bad). [lab] separates lab sessions from
/// theory, which use [accent].
@immutable
class AppPalette extends ThemeExtension<AppPalette> {
  const AppPalette({
    required this.accent,
    required this.onAccent,
    required this.accentTone,
    required this.success,
    required this.warning,
    required this.danger,
    required this.lab,
    required this.shadow,
  });

  final Color accent;
  final Color onAccent;
  final Tone accentTone;
  final Tone success;
  final Tone warning;
  final Tone danger;
  final Tone lab;

  /// Soft ambient shadow for raised surfaces; transparent in dark mode where
  /// elevation reads through borders instead.
  final Color shadow;

  static const light = AppPalette(
    accent: Color(0xFF4F46E5),
    onAccent: Color(0xFFFFFFFF),
    accentTone: Tone(
      base: Color(0xFF4F46E5),
      subtle: Color(0xFFEEF0FF),
      onSubtle: Color(0xFF3730A3),
    ),
    success: Tone(
      base: Color(0xFF16A34A),
      subtle: Color(0xFFEAF7EE),
      onSubtle: Color(0xFF166534),
    ),
    warning: Tone(
      base: Color(0xFFD97706),
      subtle: Color(0xFFFEF5E7),
      onSubtle: Color(0xFF92400E),
    ),
    danger: Tone(
      base: Color(0xFFDC2626),
      subtle: Color(0xFFFDECEC),
      onSubtle: Color(0xFF991B1B),
    ),
    lab: Tone(
      base: Color(0xFF0D9488),
      subtle: Color(0xFFE6F6F4),
      onSubtle: Color(0xFF115E59),
    ),
    shadow: Color(0x0F000000),
  );

  static const dark = AppPalette(
    accent: Color(0xFF818CF8),
    onAccent: Color(0xFF0B0B1A),
    accentTone: Tone(
      base: Color(0xFF818CF8),
      subtle: Color(0xFF1E1D3A),
      onSubtle: Color(0xFFC7CBFF),
    ),
    success: Tone(
      base: Color(0xFF4ADE80),
      subtle: Color(0xFF12261A),
      onSubtle: Color(0xFF86EFAC),
    ),
    warning: Tone(
      base: Color(0xFFFBBF24),
      subtle: Color(0xFF2A2112),
      onSubtle: Color(0xFFFCD34D),
    ),
    danger: Tone(
      base: Color(0xFFF87171),
      subtle: Color(0xFF2C1616),
      onSubtle: Color(0xFFFCA5A5),
    ),
    lab: Tone(
      base: Color(0xFF2DD4BF),
      subtle: Color(0xFF0F2624),
      onSubtle: Color(0xFF5EEAD4),
    ),
    shadow: Color(0x00000000),
  );

  @override
  AppPalette copyWith({
    Color? accent,
    Color? onAccent,
    Tone? accentTone,
    Tone? success,
    Tone? warning,
    Tone? danger,
    Tone? lab,
    Color? shadow,
  }) => AppPalette(
    accent: accent ?? this.accent,
    onAccent: onAccent ?? this.onAccent,
    accentTone: accentTone ?? this.accentTone,
    success: success ?? this.success,
    warning: warning ?? this.warning,
    danger: danger ?? this.danger,
    lab: lab ?? this.lab,
    shadow: shadow ?? this.shadow,
  );

  @override
  AppPalette lerp(AppPalette? other, double t) {
    if (other is! AppPalette) return this;
    return AppPalette(
      accent: Color.lerp(accent, other.accent, t)!,
      onAccent: Color.lerp(onAccent, other.onAccent, t)!,
      accentTone: Tone.lerp(accentTone, other.accentTone, t),
      success: Tone.lerp(success, other.success, t),
      warning: Tone.lerp(warning, other.warning, t),
      danger: Tone.lerp(danger, other.danger, t),
      lab: Tone.lerp(lab, other.lab, t),
      shadow: Color.lerp(shadow, other.shadow, t)!,
    );
  }
}

extension AppPaletteColors on FColors {
  /// The app palette, falling back to the default for this brightness when a
  /// theme (e.g. a bare forui theme in tests) doesn't carry the extension.
  AppPalette get app =>
      extensions.whereType<AppPalette>().firstOrNull ??
      (brightness == Brightness.dark ? AppPalette.dark : AppPalette.light);
}

/// Spacing scale (4pt grid). Use these instead of ad-hoc paddings so screens
/// line up with each other.
abstract final class Space {
  static const double xs = 4;
  static const double sm = 8;
  static const double md = 12;
  static const double lg = 16;
  static const double xl = 24;
  static const double xxl = 32;

  /// Horizontal inset for page content inside the shell.
  static const double page = 16;
}

/// Corner radii used across surfaces.
abstract final class Radii {
  static const double sm = 8;
  static const double md = 12;
  static const double lg = 16;
  static const double pill = 999;
}

/// Motion durations; pair with [Curves.easeOutCubic] for entrances.
abstract final class Motion {
  static const fast = Duration(milliseconds: 120);
  static const medium = Duration(milliseconds: 220);
  static const slow = Duration(milliseconds: 360);
}
