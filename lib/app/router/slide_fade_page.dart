import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';

/// Pushed-page transition in the Material "shared axis X" style: the page
/// underneath fades out while drifting left, then the new page fades in while
/// settling from the right. The two never overlap at full strength, so pages
/// without their own background don't bleed through each other.
/// Falls back to no animation when the system asks to reduce motion.
class SlideFadePage<T> extends CustomTransitionPage<T> {
  const SlideFadePage({super.key, required super.child})
    : super(
        transitionDuration: const Duration(milliseconds: 300),
        reverseTransitionDuration: const Duration(milliseconds: 250),
        transitionsBuilder: _build,
      );

  static const _travel = 28.0;

  static Widget _build(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    // Pages rely on the shell for their background; paint one so the page
    // underneath doesn't show through.
    child = ColoredBox(color: context.theme.colors.background, child: child);
    if (MediaQuery.disableAnimationsOf(context)) return child;

    return AnimatedBuilder(
      animation: Listenable.merge([animation, secondaryAnimation]),
      child: child,
      builder: (context, child) {
        final inT = animation.value;
        final outT = secondaryAnimation.value;

        final inFade = const Interval(
          0.3,
          1,
          curve: Curves.easeOut,
        ).transform(inT);
        final inShift = (1 - Curves.easeOutCubic.transform(inT)) * _travel;
        final outFade =
            1 - const Interval(0, 0.3, curve: Curves.easeIn).transform(outT);
        final outShift = -Curves.easeOutCubic.transform(outT) * _travel;

        return Opacity(
          opacity: inFade * outFade,
          child: Transform.translate(
            offset: Offset(inShift + outShift, 0),
            child: child,
          ),
        );
      },
    );
  }
}
