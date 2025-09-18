import 'package:app_settings/app_settings.dart';
import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/features/settings/presentation/widgets/pb_helper.dart';
import 'package:vitapmate/features/settings/presentation/widgets/social_avatar_update.dart';
import 'package:vitapmate/features/settings/presentation/widgets/social_username_update.dart';
import 'package:vitapmate/main.dart';

class SettingsPage extends HookConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) => Container(
    decoration: BoxDecoration(color: context.theme.colors.background),
    child: Padding(
      padding: const EdgeInsets.all(8.0),
      child: Column(
        children: [
          Expanded(
            child: Column(
              spacing: 8,
              children: [
                FTileGroup(
                  label: const Text('Vtop'),
                  children: [
                    FTile(
                      prefix: Icon(FIcons.user),
                      title: const Text('Vtop Details'),
                      suffix: Icon(FIcons.chevronRight),
                      onPress: () {
                        GoRouter.of(
                          context,
                        ).pushNamed(Paths.vtopUserManagement);
                      },
                    ),
                  ],
                ),

                FTileGroup(
                  divider: FItemDivider.indented,
                  label: const Text('Social'),
                  children: [
                    FTile(
                      prefix: Icon(FIcons.atSign),
                      title: const Text('Username'),
                      suffix: Icon(FIcons.chevronRight),
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
                      prefix: Icon(FIcons.image),
                      title: const Text('Avatar'),
                      suffix: Icon(FIcons.chevronRight),
                      onPress:
                          () => showAdaptiveDialog(
                            context: context,
                            builder:
                                (context) => PbHelper(
                                  child: (pb) => SocialAvatarUpdate(pb),
                                ),
                          ),
                    ),
                    FTile(
                      prefix: Icon(FIcons.bell),
                      title: const Text("Notifications"),
                      suffix: Icon(FIcons.dot),
                      onPress: () async {
                        var settings = await notifications.requestPermissions();
                        AppSettings.openAppSettings(
                          type: AppSettingsType.notification,
                        );
                      },
                    ),
                  ],
                ),
                FTileGroup(
                  divider: FItemDivider.indented,
                  label: const Text('Appearance'),
                  children: [
                    FTile(
                      prefix: Icon(FIcons.moon),
                      title: const Text('Dark Mode'),

                      suffix: FSwitch(
                        value: ref.watch(themeProvider) == ThemeMode.dark,
                        onChange: (value) {
                          ref.read(themeProvider.notifier).toggleTheme();
                        },
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              IconButton(
                icon: Icon(FIcons.github),
                onPressed: () {
                  launchUrl(
                    Uri.parse("https://github.com/synaptic-gg/vitap-mate"),
                  );
                },
              ),
              IconButton(
                icon: Icon(FIcons.contact),
                onPressed: () {
                  launchUrl(Uri.parse("https://bio.link/synaptic"));
                },
              ),
            ],
          ),
        ],
      ),
    ),
  );
}
