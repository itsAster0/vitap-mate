import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/ui/text_nums.dart';

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
      padding: const .symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: solid ? tone.base : tone.subtle,
        borderRadius: .circular(Radii.pill),
      ),
      child: Row(
        mainAxisSize: .min,
        children: [
          if (icon != null) ...[
            Icon(icon, size: 11, color: fg),
            const SizedBox(width: 4),
          ],
          Text(
            label,
            style: numStyle(
              fontSize: 10.5,
              height: 1.3,
              fontWeight: .w700,
              letterSpacing: 0.4,
              color: fg,
            ),
          ),
        ],
      ),
    );
  }
}
