import 'package:flutter/material.dart';
import 'package:forui/forui.dart';

class InfoDispShare extends StatelessWidget {
  const InfoDispShare({super.key});

  @override
  Widget build(BuildContext context) {
    return FButton.icon(
      onPress: () {
        showDialog(
          context: context,
          builder: (context) {
            return FDialog(
              title: Text("Timetable Sharing"),
              body: Text("""

⋅ A unique link will be generated for sharing.

⋅ You can remove or regenerate the link at any time.

⋅ Old links stop working immediately after regeneration.

⋅ By enabling this feature, you agree that your timetable will be stored on our servers — this is required for the feature to work.

⋅ Whenever you update your timetable, the version stored on the server will also update automatically.,
             """, textAlign: TextAlign.start),
              actions: [
                FButton(
                  onPress: () => Navigator.of(context).pop(),
                  child: const Text('OK'),
                ),
              ],
            );
          },
        );
      },
      child: Icon(FIcons.badgeInfo),
    );
  }
}
