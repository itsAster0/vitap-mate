import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';

/// Thin horizontal bar that animates to [value] (0–1).
class ProgressBar extends StatelessWidget {
  const ProgressBar({
    super.key,
    required this.value,
    required this.color,
    this.height = 6,
  });

  final double value;
  final Color color;
  final double height;

  @override
  Widget build(BuildContext context) => ClipRRect(
    borderRadius: .circular(height),
    child: SizedBox(
      height: height,
      child: ColoredBox(
        color: context.theme.colors.secondary,
        child: TweenAnimationBuilder<double>(
          tween: Tween(begin: 0, end: value.clamp(0.0, 1.0)),
          duration: Motion.reduced(context) ? .zero : Motion.slow * 2,
          curve: Curves.easeOutCubic,
          builder: (context, v, _) => FractionallySizedBox(
            alignment: .centerLeft,
            widthFactor: v,
            child: ColoredBox(color: color),
          ),
        ),
      ),
    ),
  );
}
