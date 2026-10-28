import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/ui/extensions.dart';

/// Faculty name on its own line with a person icon; wraps to two lines
/// instead of being cut off.
class FacultyLine extends StatelessWidget {
  const FacultyLine({super.key, required this.name});

  final String name;

  @override
  Widget build(BuildContext context) {
    final muted = context.theme.colors.mutedForeground;
    return Row(
      crossAxisAlignment: .start,
      children: [
        Padding(
          padding: const .only(top: 1),
          child: Icon(FLucideIcons.user, size: 13, color: muted),
        ),
        const SizedBox(width: Space.xs + 2),
        Expanded(
          child: Text(
            name.asPersonName,
            maxLines: 2,
            overflow: .ellipsis,
            style: context.theme.typography.body.xs.copyWith(color: muted),
          ),
        ),
      ],
    );
  }
}
