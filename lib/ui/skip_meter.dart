import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/ui/progress_bar.dart';

/// Picks the status tone for an attendance percentage and its guidance.
Tone attendanceTone(
  BuildContext context, {
  required bool safe,
  required bool atEdge,
}) {
  final palette = context.theme.colors.app;
  return !safe
      ? palette.danger
      : atEdge
      ? palette.warning
      : palette.success;
}

/// Attendance bar with a marker at the 75% line, so the gap above or below
/// the requirement is visible at a glance.
class SkipMeter extends StatelessWidget {
  const SkipMeter({
    super.key,
    required this.percent,
    required this.tone,
    this.threshold = 75,
  });

  final double percent;
  final Tone tone;
  final double threshold;

  @override
  Widget build(BuildContext context) => SizedBox(
    height: 14,
    child: LayoutBuilder(
      builder: (context, constraints) => Stack(
        clipBehavior: .none,
        alignment: .centerLeft,
        children: [
          ProgressBar(value: percent / 100, color: tone.base, height: 8),
          Positioned(
            left: constraints.maxWidth * threshold / 100 - 1,
            top: 0,
            bottom: 0,
            child: Container(
              width: 2,
              decoration: BoxDecoration(
                color: context.theme.colors.foreground.withValues(alpha: 0.7),
                borderRadius: .circular(1),
              ),
            ),
          ),
        ],
      ),
    ),
  );
}
