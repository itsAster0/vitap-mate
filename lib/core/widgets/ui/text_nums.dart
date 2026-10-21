import 'package:flutter/widgets.dart';

const _tabular = [FontFeature.tabularFigures()];

/// Digits of equal width, so numbers line up in columns and don't jitter as
/// they change.
extension NumberText on TextStyle {
  /// [copyWith] with tabular figures.
  TextStyle nums({
    double? fontSize,
    FontWeight? fontWeight,
    FontStyle? fontStyle,
    Color? color,
    double? height,
    double? letterSpacing,
    TextDecoration? decoration,
    Color? decorationColor,
  }) => copyWith(
    fontSize: fontSize,
    fontWeight: fontWeight,
    fontStyle: fontStyle,
    color: color,
    height: height,
    letterSpacing: letterSpacing,
    decoration: decoration,
    decorationColor: decorationColor,
    fontFeatures: _tabular,
  );
}

/// A [TextStyle] with tabular figures.
TextStyle numStyle({
  double? fontSize,
  FontWeight? fontWeight,
  FontStyle? fontStyle,
  Color? color,
  double? height,
  double? letterSpacing,
  TextDecoration? decoration,
  Color? decorationColor,
}) => const TextStyle().nums(
  fontSize: fontSize,
  fontWeight: fontWeight,
  fontStyle: fontStyle,
  color: color,
  height: height,
  letterSpacing: letterSpacing,
  decoration: decoration,
  decorationColor: decorationColor,
);
