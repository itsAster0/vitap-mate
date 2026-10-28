import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';
import 'package:vitapmate/ui/tone_badge.dart';

/// LAB / THEORY tag with an icon, in the lab or accent tone.
class CourseKindBadge extends StatelessWidget {
  const CourseKindBadge({super.key, required this.isLab});

  final bool isLab;

  @override
  Widget build(BuildContext context) {
    final palette = context.theme.colors.app;
    return isLab
        ? ToneBadge(
            label: 'LAB',
            icon: FLucideIcons.flaskConical,
            tone: palette.lab,
          )
        : ToneBadge(
            label: 'THEORY',
            icon: FLucideIcons.bookOpen,
            tone: palette.accentTone,
          );
  }
}
