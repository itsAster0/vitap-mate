import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/ui/app_palette.dart';

/// Grab handle shown at the top of bottom sheets.
class SheetHandle extends StatelessWidget {
  const SheetHandle({super.key});

  @override
  Widget build(BuildContext context) => Center(
    child: Container(
      margin: const .only(top: Space.sm + 2, bottom: Space.xs),
      width: 36,
      height: 4,
      decoration: BoxDecoration(
        color: context.theme.colors.border,
        borderRadius: .circular(Radii.pill),
      ),
    ),
  );
}
