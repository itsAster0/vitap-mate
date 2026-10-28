import 'dart:math' as math;

import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';

/// Circular progress ring that animates to [value] (0–1) with a centred child.
class ProgressRing extends StatelessWidget {
  const ProgressRing({
    super.key,
    required this.value,
    required this.color,
    this.size = 52,
    this.stroke = 5,
    this.marker,
    this.child,
  });

  final double value;
  final Color color;
  final double size;
  final double stroke;

  /// Optional threshold (0–1) drawn as a small tick across the ring.
  final double? marker;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final markerColor = colors.foreground.withValues(alpha: 0.6);
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: value.clamp(0.0, 1.0)),
      duration: Motion.reduced(context) ? .zero : Motion.slow * 2,
      curve: Curves.easeOutCubic,
      builder: (context, v, child) => CustomPaint(
        painter: _RingPainter(
          v,
          color,
          colors.secondary,
          stroke,
          marker,
          markerColor,
        ),
        child: child,
      ),
      child: SizedBox.square(
        dimension: size,
        child: Center(child: child),
      ),
    );
  }
}

class _RingPainter extends CustomPainter {
  _RingPainter(
    this.value,
    this.color,
    this.track,
    this.stroke,
    this.marker,
    this.markerColor,
  );

  final double value;
  final Color color;
  final Color track;
  final double stroke;
  final double? marker;
  final Color markerColor;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    final arc = rect.deflate(stroke / 2);
    final paint = Paint()
      ..style = .stroke
      ..strokeWidth = stroke
      ..strokeCap = .round;
    canvas.drawArc(arc, 0, math.pi * 2, false, paint..color = track);
    if (value > 0) {
      canvas.drawArc(
        arc,
        -math.pi / 2,
        math.pi * 2 * value,
        false,
        paint..color = color,
      );
    }
    if (marker case final marker?) {
      final angle = -math.pi / 2 + math.pi * 2 * marker;
      final center = rect.center;
      // Kept inside the stroke so it reads as a notch, not a mark beside it.
      final outer = size.width / 2;
      final inner = size.width / 2 - stroke;
      final dir = Offset(math.cos(angle), math.sin(angle));
      canvas.drawLine(
        center + dir * inner,
        center + dir * outer,
        Paint()
          ..color = markerColor
          ..strokeWidth = 1.5
          ..strokeCap = .round,
      );
    }
  }

  @override
  bool shouldRepaint(_RingPainter old) =>
      old.value != value ||
      old.color != color ||
      old.track != track ||
      old.marker != marker ||
      old.markerColor != markerColor;
}
