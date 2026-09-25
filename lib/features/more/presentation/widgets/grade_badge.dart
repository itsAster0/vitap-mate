import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';

/// Tone for a VIT grade letter: S green, A teal, B indigo, C/D amber,
/// E/F/N red — distinct so a grade mix reads at a glance.
Tone gradeTone(BuildContext context, String grade) {
  final colors = context.theme.colors;
  final palette = colors.app;
  return switch (grade.trim().toUpperCase()) {
    'S' => palette.success,
    'A' => palette.lab,
    'B' => palette.accentTone,
    'C' || 'D' => palette.warning,
    'E' || 'F' || 'N' => palette.danger,
    _ => Tone(
      base: colors.mutedForeground,
      subtle: colors.secondary,
      onSubtle: colors.mutedForeground,
    ),
  };
}

/// Rounded square holding a grade letter in its tone.
class GradeBadge extends StatelessWidget {
  const GradeBadge({super.key, required this.grade, this.size = 36});

  final String grade;
  final double size;

  @override
  Widget build(BuildContext context) {
    final tone = gradeTone(context, grade);
    final label = grade.trim().isEmpty ? '–' : grade.trim().toUpperCase();
    return Container(
      width: size,
      height: size,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        color: tone.subtle,
        borderRadius: BorderRadius.circular(Radii.sm + 2),
      ),
      child: Text(
        label,
        style: context.theme.typography.body.md.copyWith(
          fontWeight: FontWeight.w700,
          color: tone.onSubtle,
        ),
      ),
    );
  }
}
