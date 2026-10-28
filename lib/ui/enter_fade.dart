import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:vitapmate/ui/app_palette.dart';

/// Fades and lifts [child] into place once, delayed by [index] so lists
/// cascade in rather than popping.
class EnterFade extends HookWidget {
  const EnterFade({super.key, required this.child, this.index = 0});

  final Widget child;
  final int index;

  @override
  Widget build(BuildContext context) {
    final reduce = Motion.reduced(context);
    final controller = useAnimationController(duration: Motion.slow);
    useEffect(() {
      if (reduce) {
        controller.value = 1;
        return null;
      }
      final delay = Duration(milliseconds: 40 * math.min(index, 8));
      return Timer(delay, controller.forward).cancel;
    }, const []);
    final curve = useMemoized(
      () => CurvedAnimation(parent: controller, curve: Curves.easeOutCubic),
      [controller],
    );
    return FadeTransition(
      opacity: curve,
      child: SlideTransition(
        position: Tween(
          begin: const Offset(0, 0.04),
          end: Offset.zero,
        ).animate(curve),
        child: child,
      ),
    );
  }
}
