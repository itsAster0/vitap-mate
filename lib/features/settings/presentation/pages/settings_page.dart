import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/features/settings/presentation/widgets/pb_helper.dart';
import 'package:vitapmate/features/settings/presentation/widgets/social_avatar_update.dart';
import 'package:vitapmate/features/settings/presentation/widgets/social_username_update.dart';

class SettingsPage extends StatelessWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context) => Container(
    decoration: BoxDecoration(color: context.theme.colors.background),
    child: Padding(
      padding: const EdgeInsets.all(8.0),
      child: Center(
        child: Column(
          spacing: 8,
          children: [
            FTileGroup(
              label: const Text('Vtop'),
              children: [
                FTile(
                  prefixIcon: Icon(FIcons.user),
                  title: const Text('Vtop Details'),
                  suffixIcon: Icon(FIcons.chevronRight),
                  onPress: () {
                    GoRouter.of(context).pushNamed(Paths.vtopUserManagement);
                  },
                ),
              ],
            ),
            FTileGroup(
              divider: FTileDivider.indented,
              label: const Text('Social'),
              children: [
                FTile(
                  prefixIcon: Icon(FIcons.atSign),
                  title: const Text('Username'),
                  suffixIcon: Icon(FIcons.chevronRight),
                  onPress:
                      () => showAdaptiveDialog(
                        context: context,
                        builder:
                            (context) => PbHelper(
                              child: (pb) => SocialUsernameUpdate(pb),
                            ),
                      ),
                ),
                FTile(
                  prefixIcon: Icon(FIcons.image),
                  title: const Text('Avatar'),
                  suffixIcon: Icon(FIcons.chevronRight),
                  onPress:
                      () => showAdaptiveDialog(
                        context: context,
                        builder:
                            (context) =>
                                PbHelper(child: (pb) => SocialAvatarUpdate(pb)),
                      ),
                ),
              ],
            ),
          ],
        ),
      ),
    ),
  );
}
