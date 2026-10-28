import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';

/// Wraps [child] in an [FTappable] that shrinks slightly while pressed.
class PressScale extends StatelessWidget {
  const PressScale({
    super.key,
    required this.child,
    required this.onPress,
    this.semanticsLabel,
    this.scale = 0.98,
  });

  final Widget child;
  final VoidCallback onPress;
  final String? semanticsLabel;
  final double scale;

  @override
  Widget build(BuildContext context) => FTappable(
    onPress: () {
      HapticFeedback.selectionClick();
      onPress();
    },
    semanticsLabel: semanticsLabel,
    builder: (context, variants, child) => AnimatedScale(
      scale: variants.contains(FTappableVariant.pressed) ? scale : 1,
      duration: Motion.fast,
      curve: Curves.easeOut,
      child: child,
    ),
    child: child,
  );
}
