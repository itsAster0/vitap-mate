import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/ui/text_nums.dart';

/// Segmented control with a thumb that slides to the selected option.
class Segmented<T> extends StatelessWidget {
  const Segmented({
    super.key,
    required this.segments,
    required this.value,
    required this.onChanged,
    this.counts,
  });

  final List<(T, String)> segments;
  final T value;
  final ValueChanged<T> onChanged;

  /// Optional count per segment (same order), shown as a small pill.
  final List<int>? counts;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final index = segments.indexWhere((s) => s.$1 == value);
    return Container(
      height: 40,
      padding: const .all(3),
      decoration: BoxDecoration(
        color: colors.secondary,
        borderRadius: .circular(Radii.md),
      ),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final width = constraints.maxWidth / segments.length;
          return Stack(
            children: [
              AnimatedPositioned(
                duration: Motion.reduced(context) ? .zero : Motion.medium,
                curve: Curves.easeOutCubic,
                left: width * index,
                top: 0,
                bottom: 0,
                width: width,
                // Page background with a border reads as a distinct raised
                // chip on the track in both themes.
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: colors.background,
                    borderRadius: .circular(Radii.sm + 1),
                    border: .all(color: colors.border),
                    boxShadow: [
                      BoxShadow(
                        color: colors.app.shadow,
                        blurRadius: 6,
                        offset: const Offset(0, 1),
                      ),
                    ],
                  ),
                ),
              ),
              Row(
                children: [
                  for (final (i, (segment, label)) in segments.indexed)
                    Expanded(
                      child: _segment(
                        context,
                        segment,
                        label,
                        count: switch (counts) {
                          final c? when i < c.length => c[i],
                          _ => null,
                        },
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

  Widget _segment(BuildContext context, T segment, String label, {int? count}) {
    final colors = context.theme.colors;
    final selected = segment == value;
    return FTappable(
      selected: selected,
      semanticsLabel: label,
      onPress: () {
        if (!selected) HapticFeedback.selectionClick();
        onChanged(segment);
      },
      child: Center(
        child: Row(
          mainAxisSize: .min,
          children: [
            // A label too long for its segment ends in an ellipsis instead
            // of overflowing.
            Flexible(
              child: AnimatedDefaultTextStyle(
                duration: Motion.medium,
                style: context.theme.typography.body.sm.copyWith(
                  fontWeight: selected ? .w600 : .w500,
                  color: selected ? colors.foreground : colors.mutedForeground,
                ),
                child: Text(label, maxLines: 1, overflow: .ellipsis),
              ),
            ),
            if (count != null) ...[
              const SizedBox(width: 6),
              AnimatedContainer(
                duration: Motion.medium,
                padding: const .symmetric(horizontal: 6, vertical: 1),
                decoration: BoxDecoration(
                  color: selected
                      ? colors.app.accentTone.subtle
                      : colors.background.withValues(alpha: 0.5),
                  borderRadius: .circular(Radii.pill),
                ),
                child: Text(
                  '$count',
                  style: numStyle(
                    fontSize: 11,
                    height: 1.3,
                    fontWeight: .w700,
                    color: selected
                        ? colors.app.accentTone.onSubtle
                        : colors.mutedForeground,
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
