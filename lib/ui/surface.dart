import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/ui/press_scale.dart';

/// The standard raised surface: theme background, hairline border, large
/// radius and a soft shadow (light mode only). Pass [onPress] to make it
/// tappable with a gentle press-scale.
class Surface extends StatelessWidget {
  const Surface({
    super.key,
    required this.child,
    this.onPress,
    this.padding = const .all(Space.lg),
    this.color,
    this.borderColor,
    this.semanticsLabel,
  });

  final Widget child;
  final VoidCallback? onPress;
  final EdgeInsetsGeometry padding;
  final Color? color;
  final Color? borderColor;
  final String? semanticsLabel;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final box = DecoratedBox(
      decoration: BoxDecoration(
        color: color ?? colors.card,
        borderRadius: .circular(Radii.lg),
        border: .all(color: borderColor ?? colors.border),
        boxShadow: [
          BoxShadow(
            color: colors.app.shadow,
            blurRadius: 12,
            offset: const Offset(0, 4),
          ),
        ],
      ),
      child: Padding(padding: padding, child: child),
    );
    return switch (onPress) {
      null => box,
      final onPress => PressScale(
        onPress: onPress,
        semanticsLabel: semanticsLabel,
        child: box,
      ),
    };
  }
}
