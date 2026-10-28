import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';

/// Placeholder block that shimmers while content loads.
class Skeleton extends HookWidget {
  const Skeleton({
    super.key,
    this.width,
    this.height = 14,
    this.radius = Radii.sm,
  });

  final double? width;
  final double height;
  final double radius;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final controller = useAnimationController(
      duration: const Duration(milliseconds: 1100),
    );
    final reduce = Motion.reduced(context);
    useEffect(() {
      if (!reduce) controller.repeat(reverse: true);
      return null;
    }, [reduce]);
    final pulse = useAnimation(controller);
    return Container(
      width: width,
      height: height,
      decoration: BoxDecoration(
        color: Color.lerp(
          colors.secondary,
          colors.muted,
          pulse,
        )!.withValues(alpha: 0.7 + 0.3 * pulse),
        borderRadius: .circular(radius),
      ),
    );
  }
}
