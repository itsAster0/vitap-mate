import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/text_nums.dart';
import 'package:vitapmate/ui/ui.dart';

/// A course card: a coloured [stripe] down the left, [children] beside it.
class StripeCard extends StatelessWidget {
  const StripeCard({
    super.key,
    required this.stripe,
    required this.children,
    this.onPress,
    this.semanticsLabel,
  });

  final Color stripe;
  final List<Widget> children;
  final VoidCallback? onPress;
  final String? semanticsLabel;

  @override
  Widget build(BuildContext context) => Surface(
    padding: .zero,
    semanticsLabel: semanticsLabel,
    onPress: onPress,
    child: ClipRRect(
      borderRadius: .circular(Radii.lg - 1),
      child: IntrinsicHeight(
        child: Row(
          crossAxisAlignment: .stretch,
          children: [
            Container(width: 4, color: stripe),
            Expanded(
              child: Padding(
                padding: const .fromLTRB(
                  Space.md + 2,
                  Space.md + 2,
                  Space.md + 2,
                  Space.md,
                ),
                child: Column(crossAxisAlignment: .start, children: children),
              ),
            ),
          ],
        ),
      ),
    ),
  );
}

/// A centred "16 left  ·  8 to spare  ·  95% max" line under a hairline.
/// Each part is a muted lead, a bold value and a label; a null lead or value
/// is left out.
class OutlookLine extends StatelessWidget {
  const OutlookLine(this.parts, {super.key, this.alert});

  final List<(String?, String?, String)> parts;

  /// Colours the values and the value-less labels when set.
  final Color? alert;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final base = context.theme.typography.body.xs.nums(
      color: colors.mutedForeground,
    );
    final strong = base.copyWith(
      fontWeight: .w600,
      color: alert ?? colors.foreground,
    );
    return Column(
      children: [
        Container(height: 1, color: colors.border),
        const SizedBox(height: Space.sm),
        Text.rich(
          TextSpan(
            children: [
              for (final (i, (lead, value, label)) in parts.indexed) ...[
                if (i > 0) TextSpan(text: '  ·  ', style: base),
                if (lead != null) TextSpan(text: lead, style: base),
                if (value != null) TextSpan(text: value, style: strong),
                TextSpan(
                  text: label,
                  style: alert != null && value == null
                      ? base.copyWith(color: alert)
                      : base,
                ),
              ],
            ],
          ),
          textAlign: .center,
          maxLines: 1,
          overflow: .ellipsis,
        ),
      ],
    );
  }
}

/// A small filled circle, or a hollow one with a [ring].
class Dot extends StatelessWidget {
  const Dot({super.key, this.size = 5, this.color, this.ring});

  final double size;
  final Color? color;
  final Color? ring;

  @override
  Widget build(BuildContext context) => Container(
    width: size,
    height: size,
    decoration: BoxDecoration(
      color: color,
      shape: .circle,
      border: ring == null ? null : .all(color: ring!, width: 1.2),
    ),
  );
}
