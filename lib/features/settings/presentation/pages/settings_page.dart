import 'package:app_settings/app_settings.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/features/background/controller.dart';
import 'package:vitapmate/features/settings/presentation/widgets/pb_helper.dart';
import 'package:vitapmate/features/settings/presentation/widgets/social_avatar_update.dart';
import 'package:vitapmate/features/settings/presentation/widgets/social_username_update.dart';
import 'package:vitapmate/main.dart';

class SettingsPage extends HookConsumerWidget {
  const SettingsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final show15 = useState(false);
    final field15e = useTextEditingController();

    final backgroundSync = [
      FSelectTile(title: Text("Disable"), value: Duration(seconds: 0)),
      if (show15.value)
        FSelectTile(title: Text("15 Minutes"), value: Duration(minutes: 15)),
      if (show15.value)
        FSelectTile(title: Text("1 hour"), value: Duration(hours: 1)),
      FSelectTile(title: Text("3 hours"), value: Duration(hours: 3)),
      FSelectTile(title: Text("6 hours"), value: Duration(hours: 6)),
      FSelectTile(title: Text("12 hours"), value: Duration(hours: 12)),
      FSelectTile(title: Text("24 hours"), value: Duration(hours: 24)),
    ];
    final initialValSync =
        ref.watch(backgroundSyncProvider).value?.freq ?? Duration(seconds: 0);
    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (initialValSync == Duration(minutes: 15) ||
            initialValSync == Duration(hours: 1)) {
          show15.value = true;
        } else {
          show15.value = false;
        }
      });
      return null;
    }, [initialValSync]);
    return Container(
      decoration: BoxDecoration(color: context.theme.colors.background),
      child: Padding(
        padding: const EdgeInsets.all(8.0),
        child: Column(
          children: [
            Expanded(
              child: SingleChildScrollView(
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
                        FTile(
                          prefix: Icon(FIcons.calendarDays),
                          title: const Text('Merge Labs'),
                          suffix: FSwitch(
                            value: ref.watch(mergeTTProvider),
                            onChange: (value) {
                              ref.read(toggleMergeTTProvider);
                            },
                          ),
                        ),
                        FTile(
                          prefix: Icon(FIcons.userCheck),
                          title: const Text('Show b/w Exams'),
                          suffix: FSwitch(
                            value: ref.watch(btwExamsProvider),
                            onChange: (value) {
                              ref.read(toggleBTWExamsProvider);
                            },
                          ),
                        ),
                        FSelectMenuTile(
                          prefix: Icon(FIcons.folderSync),
                          title: FTappable(
                            onLongPress: () {
                              showFDialog(
                                context: context,
                                builder:
                                    (context, style, animation) => FDialog(
                                      animation: animation,
                                      direction: Axis.horizontal,
                                      title: const Text(
                                        'Are you absolutely sure?',
                                      ),
                                      body: FTextField(controller: field15e),
                                      actions: [
                                        FButton(
                                          child: const Text('Continue'),
                                          onPress: () {
                                            if (field15e.text
                                                    .trim()
                                                    .toLowerCase() ==
                                                "why") {
                                              show15.value = true;
                                            }

                                            Navigator.of(context).pop();
                                          },
                                        ),
                                      ],
                                    ),
                              );
                            },
                            child: Text('Background Sync'),
                          ),

                          onChange:
                              (value) => {
                                ref
                                    .read(backgroundSyncProvider.notifier)
                                    .updateFreq(value.first),
                              },
                          initialValue: initialValSync,
                          menu: backgroundSync,
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
                      ],
                    ),
                    FTileGroup(
                      divider: FItemDivider.indented,
                      label: const Text('App Settings'),
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
                        FTile(
                          prefix: Icon(FIcons.wifi),
                          title: const Text('Show Wi-Fi Card'),

                          suffix: FSwitch(
                            value: ref.watch(wificardSettingProvider),
                            onChange: (value) {
                              ref.read(toggleWificardProvider);
                            },
                          ),
                        ),
                        FTile(
                          prefix: Icon(FIcons.bell),
                          title: const Text("Notifications"),
                          suffix: Icon(FIcons.dot),
                          onPress: () async {
                            var settings =
                                await notifications.requestPermissions();
                            AppSettings.openAppSettings(
                              type: AppSettingsType.notification,
                            );
                          },
                        ),
                      ],
                    ),
                  ],
                ),
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
}
