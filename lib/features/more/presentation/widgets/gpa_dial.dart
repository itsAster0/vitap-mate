import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';

class GpaDial extends ConsumerWidget {
  final double value;
  final double size;
  final String label;
  final String? caption;

  const GpaDial({
    super.key,
    required this.value,
    this.size = 190,
    this.label = 'GPA',
    this.caption,
  });

  Color _colorFor(double v) {
    if (v >= 8) return const Color(0xFF2E7D32);
    if (v >= 6.5) return const Color(0xFF9CCC65);
    if (v >= 5) return const Color(0xFFE65100);
    return const Color(0xFFD32F2F);
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final darkMode = ref.watch(themeProvider) == ThemeMode.dark;
    final clamped = value.clamp(0.0, 10.0);

    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: clamped),
      duration: const Duration(milliseconds: 650),
      curve: Curves.easeOutCubic,
      builder: (context, animValue, _) {
        final color = _colorFor(animValue);
        return Container(
          width: size,
          height: size,
          padding: const EdgeInsets.all(10),
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            color: darkMode ? context.theme.colors.primaryForeground : null,
            gradient: darkMode
                ? null
                : LinearGradient(
                    colors: [
                      color.withValues(alpha: 0.08),
                      Colors.white.withValues(alpha: 0.02),
                    ],
                    begin: Alignment.topLeft,
                    end: Alignment.bottomRight,
                  ),
            boxShadow: const [
              BoxShadow(color: Color(0x1A000000), blurRadius: 14),
            ],
          ),
          child: CustomPaint(
            painter: _RingPainter(
              progress: animValue / 10,
              color: color,
              darkMode: darkMode,
              trackColor: darkMode
                  ? context.theme.colors.border
                  : const Color(0xFFEEEEEE),
            ),
            child: Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Text(
                    animValue.toStringAsFixed(2),
                    style: TextStyle(
                      fontSize: size * 0.19,
                      fontWeight: FontWeight.w800,
                      letterSpacing: -1,
                      color: darkMode ? context.theme.colors.primary : color,
                      fontFeatures: [const FontFeature.tabularFigures()],
                    ),
                  ),
                  Text(
                    label,
                    style: TextStyle(
                      fontSize: size * 0.062,
                      fontWeight: FontWeight.w700,
                      letterSpacing: 1.5,
                      color: context.theme.colors.mutedForeground,
                    ),
                  ),
                  if (caption != null && caption!.isNotEmpty) ...[
                    const SizedBox(height: 3),
                    Text(
                      caption!,
                      style: TextStyle(
                        fontSize: size * 0.058,
                        fontWeight: FontWeight.w600,
                        color: context.theme.colors.mutedForeground,
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

class _RingPainter extends CustomPainter {
  final double progress;
  final Color color;
  final bool darkMode;
  final Color trackColor;

  _RingPainter({
    required this.progress,
    required this.color,
    required this.darkMode,
    required this.trackColor,
  });

  @override
  void paint(Canvas canvas, Size size) {
    final stroke = size.width * 0.075;
    final inset = stroke / 2 + size.width * 0.02;
    final rect = Rect.fromLTWH(inset, inset, size.width - stroke - 4, size.height - stroke - 4);
    const startAngle = -225 * math.pi / 180;
    const sweepTotal = 270 * math.pi / 180;

    final track = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..strokeCap = StrokeCap.round
      ..color = trackColor;

    canvas.drawArc(rect, startAngle, sweepTotal, false, track);

    if (progress <= 0) return;

    final arc = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..strokeCap = StrokeCap.round
      ..shader = SweepGradient(
        startAngle: startAngle,
        endAngle: startAngle + sweepTotal,
        colors: [color.withValues(alpha: 0.55), color],
        transform: const GradientRotation(-math.pi / 12),
      ).createShader(rect);

    canvas.drawArc(rect, startAngle, sweepTotal * progress.clamp(0.0, 1.0), false, arc);
  }

  @override
  bool shouldRepaint(_RingPainter old) =>
      old.progress != progress || old.color != color || old.trackColor != trackColor;
}
