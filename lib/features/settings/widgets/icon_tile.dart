import 'package:flutter/widgets.dart';
import 'package:vitapmate/ui/ui.dart';

/// A small tinted icon at the start of a settings row.
class IconTile extends StatelessWidget {
  const IconTile({super.key, required this.icon, required this.tone});

  final IconData icon;
  final Tone tone;

  @override
  Widget build(BuildContext context) => Container(
    width: 30,
    height: 30,
    decoration: BoxDecoration(
      color: tone.subtle,
      borderRadius: .circular(Radii.sm + 1),
    ),
    child: Icon(icon, size: 16, color: tone.onSubtle),
  );
}
