import 'package:flutter/widgets.dart';
import 'package:vitapmate/ui/ui.dart';

/// The tinted square icon the Server page's tiles lead with.
class ServerIconBox extends StatelessWidget {
  const ServerIconBox({super.key, required this.icon, required this.tone});

  final IconData icon;
  final Tone tone;

  @override
  Widget build(BuildContext context) => Container(
    width: 30,
    height: 30,
    decoration: BoxDecoration(
      color: tone.subtle,
      borderRadius: BorderRadius.circular(Radii.sm + 1),
    ),
    child: Icon(icon, size: 16, color: tone.onSubtle),
  );
}
