import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/ui/text_nums.dart';

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
    final body = context.theme.typography.body;
    return Row(
      children: [
        Expanded(
          child: Column(
            crossAxisAlignment: .start,
            children: [
              Text(
                label,
                style: body.sm.copyWith(
                  fontWeight: .w600,
                  color: colors.foreground,
                ),
              ),
              if (caption case final caption?)
                Text(
                  caption,
                  style: body.xs.copyWith(color: colors.mutedForeground),
                ),
            ],
          ),
        ),
        FButton.icon(
          variant: .outline,
          size: .sm,
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
              textAlign: .center,
              style: body.lg.nums(
                fontWeight: .w700,
                color: enabled ? colors.foreground : colors.mutedForeground,
              ),
            ),
          ),
        ),
        FButton.icon(
          variant: .outline,
          size: .sm,
          semanticsLabel: 'Increase $label',
          onPress: enabled ? () => onChanged(value + 1) : null,
          child: const Icon(FLucideIcons.plus),
        ),
      ],
    );
  }
}
