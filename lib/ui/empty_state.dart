import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';

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
    final body = context.theme.typography.body;
    return Padding(
      padding: const .symmetric(horizontal: Space.xl, vertical: 48),
      child: Column(
        mainAxisSize: .min,
        children: [
          Container(
            width: 56,
            height: 56,
            decoration: BoxDecoration(
              color: colors.secondary,
              borderRadius: .circular(Radii.lg),
            ),
            child: Icon(icon, size: 26, color: colors.mutedForeground),
          ),
          const SizedBox(height: Space.lg),
          Text(
            title,
            textAlign: .center,
            style: body.md.copyWith(
              fontWeight: .w700,
              color: colors.foreground,
            ),
          ),
          if (message case final message?) ...[
            const SizedBox(height: Space.xs),
            Text(
              message,
              textAlign: .center,
              style: body.sm.copyWith(color: colors.mutedForeground),
            ),
          ],
          if (action case final action?) ...[
            const SizedBox(height: Space.lg),
            action,
          ],
        ],
      ),
    );
  }
}
