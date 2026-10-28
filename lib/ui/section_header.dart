import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';

/// Section title with an optional trailing widget (count, action).
class SectionHeader extends StatelessWidget {
  const SectionHeader({super.key, required this.title, this.trailing});

  final String title;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final body = context.theme.typography.body;
    return Padding(
      padding: const .fromLTRB(2, Space.lg, 2, Space.sm + 2),
      child: Row(
        children: [
          Expanded(
            child: Text(
              title,
              style: body.lg.copyWith(
                fontWeight: .w700,
                color: colors.foreground,
              ),
            ),
          ),
          if (trailing case final trailing?)
            DefaultTextStyle.merge(
              style: body.sm.copyWith(color: colors.mutedForeground),
              child: trailing,
            ),
        ],
      ),
    );
  }
}
