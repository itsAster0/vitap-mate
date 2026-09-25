import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/theme/app_palette.dart';

export 'package:vitapmate/core/theme/app_palette.dart';

bool _reduceMotion(BuildContext context) =>
    MediaQuery.maybeDisableAnimationsOf(context) ?? false;

/// The standard raised surface: theme background, hairline border, large
/// radius and a soft shadow (light mode only). Pass [onPress] to make it
/// tappable with a gentle press-scale.
class Surface extends StatelessWidget {
  const Surface({
    super.key,
    required this.child,
    this.onPress,
    this.padding = const EdgeInsets.all(Space.lg),
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
        borderRadius: BorderRadius.circular(Radii.lg),
        border: Border.all(color: borderColor ?? colors.border),
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
    if (onPress == null) return box;
    return PressScale(
      onPress: onPress!,
      semanticsLabel: semanticsLabel,
      child: box,
    );
  }
}

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
  Widget build(BuildContext context) {
    return FTappable(
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
}

/// Small uppercase label on a tinted fill, e.g. NOW, LAB, 92%.
class ToneBadge extends StatelessWidget {
  const ToneBadge({
    super.key,
    required this.label,
    required this.tone,
    this.icon,
    this.solid = false,
  });

  /// A neutral badge using the theme's muted colours.
  factory ToneBadge.neutral(
    BuildContext context,
    String label, {
    IconData? icon,
  }) {
    final colors = context.theme.colors;
    return ToneBadge(
      label: label,
      icon: icon,
      tone: Tone(
        base: colors.mutedForeground,
        subtle: colors.secondary,
        onSubtle: colors.mutedForeground,
      ),
    );
  }

  final String label;
  final Tone tone;
  final IconData? icon;
  final bool solid;

  @override
  Widget build(BuildContext context) {
    final fg = solid ? context.theme.colors.background : tone.onSubtle;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: solid ? tone.base : tone.subtle,
        borderRadius: BorderRadius.circular(Radii.pill),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 11, color: fg),
            const SizedBox(width: 4),
          ],
          Text(
            label,
            style: TextStyle(
              fontSize: 10.5,
              height: 1.3,
              fontWeight: FontWeight.w700,
              letterSpacing: 0.4,
              color: fg,
              fontFeatures: const [FontFeature.tabularFigures()],
            ),
          ),
        ],
      ),
    );
  }
}

/// Section title with an optional trailing widget (count, action).
class SectionHeader extends StatelessWidget {
  const SectionHeader({super.key, required this.title, this.trailing});

  final String title;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Padding(
      padding: const EdgeInsets.fromLTRB(2, Space.xl, 2, Space.md),
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              style: context.theme.typography.body.lg.copyWith(
                fontWeight: FontWeight.w700,
                color: colors.foreground,
              ),
            ),
          ),
          if (trailing != null)
            DefaultTextStyle.merge(
              style: context.theme.typography.body.sm.copyWith(
                color: colors.mutedForeground,
              ),
              child: trailing!,
            ),
        ],
      ),
    );
  }
}

/// Circular progress ring that animates to [value] (0–1) with a centred child.
class ProgressRing extends HookWidget {
  const ProgressRing({
    super.key,
    required this.value,
    required this.color,
    this.size = 52,
    this.stroke = 5,
    this.child,
  });

  final double value;
  final Color color;
  final double size;
  final double stroke;
  final Widget? child;

  @override
  Widget build(BuildContext context) {
    final track = context.theme.colors.secondary;
    final target = value.clamp(0.0, 1.0);
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: target),
      duration: _reduceMotion(context) ? Duration.zero : Motion.slow * 2,
      curve: Curves.easeOutCubic,
      builder: (context, v, child) => CustomPaint(
        painter: _RingPainter(v, color, track, stroke),
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
  _RingPainter(this.value, this.color, this.track, this.stroke);

  final double value;
  final Color color;
  final Color track;
  final double stroke;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    final arc = rect.deflate(stroke / 2);
    final paint = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = stroke
      ..strokeCap = StrokeCap.round;
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
  }

  @override
  bool shouldRepaint(_RingPainter old) =>
      old.value != value || old.color != color || old.track != track;
}

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
  Widget build(BuildContext context) {
    return ClipRRect(
      borderRadius: BorderRadius.circular(height),
      child: SizedBox(
        height: height,
        child: ColoredBox(
          color: context.theme.colors.secondary,
          child: TweenAnimationBuilder<double>(
            tween: Tween(begin: 0, end: value.clamp(0.0, 1.0)),
            duration: _reduceMotion(context) ? Duration.zero : Motion.slow * 2,
            curve: Curves.easeOutCubic,
            builder: (context, v, _) => FractionallySizedBox(
              alignment: Alignment.centerLeft,
              widthFactor: v,
              child: ColoredBox(color: color),
            ),
          ),
        ),
      ),
    );
  }
}

/// A number that counts up to [value] when first shown or when it changes.
class CountUp extends StatelessWidget {
  const CountUp({
    super.key,
    required this.value,
    required this.style,
    this.decimals = 0,
    this.suffix = '',
  });

  final double value;
  final TextStyle style;
  final int decimals;
  final String suffix;

  @override
  Widget build(BuildContext context) {
    return TweenAnimationBuilder<double>(
      tween: Tween(begin: 0, end: value),
      duration: _reduceMotion(context) ? Duration.zero : Motion.slow * 2,
      curve: Curves.easeOutCubic,
      builder: (context, v, _) => Text(
        '${v.toStringAsFixed(decimals)}$suffix',
        style: style.copyWith(
          fontFeatures: const [FontFeature.tabularFigures()],
        ),
      ),
    );
  }
}

/// Fades and lifts [child] into place once, delayed by [index] so lists
/// cascade in rather than popping.
class EnterFade extends HookWidget {
  const EnterFade({super.key, required this.child, this.index = 0});

  final Widget child;
  final int index;

  @override
  Widget build(BuildContext context) {
    final reduce = _reduceMotion(context);
    final controller = useAnimationController(duration: Motion.slow);
    useEffect(() {
      if (reduce) {
        controller.value = 1;
        return null;
      }
      final delay = Duration(milliseconds: 40 * math.min(index, 8));
      final timer = Timer(delay, controller.forward);
      return timer.cancel;
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
    final reduce = _reduceMotion(context);
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
        borderRadius: BorderRadius.circular(radius),
      ),
    );
  }
}

/// A card-shaped skeleton, repeated [count] times, for list screens.
class SkeletonList extends StatelessWidget {
  const SkeletonList({super.key, this.count = 5, this.height = 96});

  final int count;
  final double height;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        for (var i = 0; i < count; i++)
          Padding(
            padding: const EdgeInsets.only(bottom: Space.md),
            child: Surface(
              child: SizedBox(
                height: height - Space.lg * 2,
                child: const Row(
                  children: [
                    Skeleton(width: 44, height: 44, radius: Radii.md),
                    SizedBox(width: Space.md),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          Skeleton(width: 180, height: 14),
                          SizedBox(height: Space.sm),
                          Skeleton(width: 110, height: 11),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
      ],
    );
  }
}

/// Centered icon + title + message, with an optional action.
class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.icon,
    required this.title,
    this.message,
    this.action,
  });

  final IconData icon;
  final String title;
  final String? message;
  final Widget? action;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: Space.xl, vertical: 48),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 56,
            height: 56,
            decoration: BoxDecoration(
              color: colors.secondary,
              borderRadius: BorderRadius.circular(Radii.lg),
            ),
            child: Icon(icon, size: 26, color: colors.mutedForeground),
          ),
          const SizedBox(height: Space.lg),
          Text(
            title,
            textAlign: TextAlign.center,
            style: context.theme.typography.body.md.copyWith(
              fontWeight: FontWeight.w700,
              color: colors.foreground,
            ),
          ),
          if (message != null) ...[
            const SizedBox(height: Space.xs),
            Text(
              message!,
              textAlign: TextAlign.center,
              style: context.theme.typography.body.sm.copyWith(
                color: colors.mutedForeground,
              ),
            ),
          ],
          if (action != null) ...[const SizedBox(height: Space.lg), action!],
        ],
      ),
    );
  }
}

/// Segmented control with a thumb that slides to the selected option.
class Segmented<T> extends StatelessWidget {
  const Segmented({
    super.key,
    required this.segments,
    required this.value,
    required this.onChanged,
  });

  final List<(T, String)> segments;
  final T value;
  final ValueChanged<T> onChanged;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final index = segments.indexWhere((s) => s.$1 == value);
    return Container(
      height: 40,
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(
        color: colors.secondary,
        borderRadius: BorderRadius.circular(Radii.md),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth / segments.length;
          return Stack(
            children: [
              AnimatedPositioned(
                duration: _reduceMotion(context)
                    ? Duration.zero
                    : Motion.medium,
                curve: Curves.easeOutCubic,
                left: width * index,
                top: 0,
                bottom: 0,
                width: width,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: colors.card,
                    borderRadius: BorderRadius.circular(Radii.sm + 1),
                    boxShadow: const [
                      BoxShadow(
                        color: Color(0x14000000),
                        blurRadius: 4,
                        offset: Offset(0, 1),
                      ),
                    ],
                  ),
                ),
              ),
              Row(
                children: [
                  for (final (segment, label) in segments)
                    Expanded(
                      child: FTappable(
                        selected: segment == value,
                        semanticsLabel: label,
                        onPress: () {
                          if (segment != value) HapticFeedback.selectionClick();
                          onChanged(segment);
                        },
                        child: Center(
                          child: AnimatedDefaultTextStyle(
                            duration: Motion.medium,
                            style: context.theme.typography.body.sm.copyWith(
                              fontWeight: segment == value
                                  ? FontWeight.w600
                                  : FontWeight.w500,
                              color: segment == value
                                  ? colors.foreground
                                  : colors.mutedForeground,
                            ),
                            child: Text(label, maxLines: 1),
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ],
          );
        },
      ),
    );
  }
}

/// Label with a − value + control.
class CountStepper extends StatelessWidget {
  const CountStepper({
    super.key,
    required this.label,
    required this.value,
    required this.onChanged,
    this.min = 0,
    this.enabled = true,
    this.caption,
  });

  final String label;
  final String? caption;
  final int value;
  final int min;
  final bool enabled;
  final ValueChanged<int> onChanged;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Row(
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                label,
                style: context.theme.typography.body.sm.copyWith(
                  fontWeight: FontWeight.w600,
                  color: colors.foreground,
                ),
              ),
              if (caption != null)
                Text(
                  caption!,
                  style: context.theme.typography.body.xs.copyWith(
                    color: colors.mutedForeground,
                  ),
                ),
            ],
          ),
        ),
        FButton.icon(
          variant: FButtonVariant.outline,
          size: FButtonSizeVariant.sm,
          semanticsLabel: 'Decrease $label',
          onPress: enabled && value > min ? () => onChanged(value - 1) : null,
          child: const Icon(FLucideIcons.minus),
        ),
        SizedBox(
          width: 44,
          child: AnimatedSwitcher(
            duration: Motion.fast,
            transitionBuilder: (child, animation) =>
                ScaleTransition(scale: animation, child: child),
            child: Text(
              '$value',
              key: ValueKey(value),
              textAlign: TextAlign.center,
              style: context.theme.typography.body.lg.copyWith(
                fontWeight: FontWeight.w700,
                color: enabled ? colors.foreground : colors.mutedForeground,
                fontFeatures: const [FontFeature.tabularFigures()],
              ),
            ),
          ),
        ),
        FButton.icon(
          variant: FButtonVariant.outline,
          size: FButtonSizeVariant.sm,
          semanticsLabel: 'Increase $label',
          onPress: enabled ? () => onChanged(value + 1) : null,
          child: const Icon(FLucideIcons.plus),
        ),
      ],
    );
  }
}

/// Grab handle shown at the top of bottom sheets.
class SheetHandle extends StatelessWidget {
  const SheetHandle({super.key});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Container(
        margin: const EdgeInsets.only(top: Space.sm + 2, bottom: Space.xs),
        width: 36,
        height: 4,
        decoration: BoxDecoration(
          color: context.theme.colors.border,
          borderRadius: BorderRadius.circular(Radii.pill),
        ),
      ),
    );
  }
}

/// Picks the status tone for an attendance percentage and its guidance.
Tone attendanceTone(
  BuildContext context, {
  required bool safe,
  required bool atEdge,
}) {
  final palette = context.theme.colors.app;
  if (!safe) return palette.danger;
  if (atEdge) return palette.warning;
  return palette.success;
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
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return SizedBox(
      height: 14,
      child: LayoutBuilder(
        builder: (context, constraints) {
          final markerX = constraints.maxWidth * threshold / 100;
          return Stack(
            clipBehavior: Clip.none,
            alignment: Alignment.centerLeft,
            children: [
              ProgressBar(value: percent / 100, color: tone.base, height: 8),
              Positioned(
                left: markerX - 1,
                top: 0,
                bottom: 0,
                child: Container(
                  width: 2,
                  decoration: BoxDecoration(
                    color: colors.foreground.withValues(alpha: 0.7),
                    borderRadius: BorderRadius.circular(1),
                  ),
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

/// LAB / THEORY tag with an icon, in the lab or accent tone.
class CourseKindBadge extends StatelessWidget {
  const CourseKindBadge({super.key, required this.isLab});

  final bool isLab;

  @override
  Widget build(BuildContext context) {
    final palette = context.theme.colors.app;
    return ToneBadge(
      label: isLab ? 'LAB' : 'THEORY',
      icon: isLab ? FLucideIcons.flaskConical : FLucideIcons.bookOpen,
      tone: isLab ? palette.lab : palette.accentTone,
    );
  }
}

/// Faculty name on its own line with a person icon; wraps to two lines
/// instead of being cut off.
class FacultyLine extends StatelessWidget {
  const FacultyLine({super.key, required this.name});

  final String name;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(top: 1),
          child: Icon(
            FLucideIcons.user,
            size: 13,
            color: colors.mutedForeground,
          ),
        ),
        const SizedBox(width: Space.xs + 2),
        Expanded(
          child: Text(
            name.trim(),
            maxLines: 2,
            overflow: TextOverflow.ellipsis,
            style: context.theme.typography.body.xs.copyWith(
              color: colors.mutedForeground,
            ),
          ),
        ),
      ],
    );
  }
}
