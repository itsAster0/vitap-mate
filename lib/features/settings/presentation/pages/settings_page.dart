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
import 'package:vitapmate/features/settings/presentation/pages/user_management.dart';

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
    return SingleChildScrollView(
      child: Column(
        spacing: 8,
        children: [
          FTileGroup(
            label: const Text('Vtop'),
            children: [
              FTile(
                prefix: Icon(FIcons.calendarDays),
                title: const Text('Merge Labs'),
                suffix: FSwitch(
                  value: ref.watch(mergeTTProvider),
                  onChange: (value) {
                    setMergeTT(ref, value);
                  },
                ),
              ),
              FTile(
                prefix: Icon(FIcons.userCheck),
                title: const Text('Show b/w Exams'),
                suffix: FSwitch(
                  value: ref.watch(btwExamsProvider),
                  onChange: (value) {
                    setbtwExam(ref, value);
                  },
                ),
              ),
              FTile(
                prefix: Icon(FIcons.refreshCcw),
                title: const Text('Auto Refresh'),
                suffix: FSwitch(
                  value: ref.watch(autoRefreshProvider),
                  onChange: (value) {
                    setautoRefresh(ref, value);
                  },
                ),
              ),
              FSelectMenuTile(
                prefix: Icon(FIcons.folderSync),
                title: FTappable(
                  onLongPress: () {
                    showFDialog(
                      context: context,
                      builder: (context, style, animation) => FDialog(
                        animation: animation,
                        direction: Axis.horizontal,
                        title: const Text('Are you absolutely sure?'),
                        body: FTextField(
                          control: FTextFieldControl.managed(
                            controller: field15e,
                          ),
                        ),
                        actions: [
                          FButton(
                            child: const Text('Continue'),
                            onPress: () {
                              if (field15e.text.trim().toLowerCase() == "why") {
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

                selectControl: FMultiValueControl.managedRadio(
                  initial: initialValSync,
                  onChange: (value) {
                    final selected = value.isEmpty ? null : value.first;
                    if (selected != null) {
                      // ref
                      //     .read(backgroundSyncProvider.notifier)
                      //     .updateFreq(selected);
                    }
                  },
                ),
                menu: backgroundSync,
              ),
            ],
          ),
          const UserBox(),

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
                prefix: Icon(FIcons.bell),
                title: const Text("Notification Management"),
                suffix: Icon(FIcons.chevronRight),
                onPress: () {
                  GoRouter.of(context).pushNamed(Paths.notificationManagement);
                },
              ),
              FTile(
                prefix: const Icon(Icons.receipt_long_outlined),
                title: const Text("Logs"),
                suffix: Icon(FIcons.chevronRight),
                onPress: () {
                  GoRouter.of(context).pushNamed(Paths.logs);
                },
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              IconButton(
                icon: Icon(FIcons.github),
                onPressed: () {
                  launchUrl(
                    Uri.parse("https://github.com/itsKryxen/vitap-mate"),
                  );
                },
              ),
              IconButton(
                icon: Icon(FIcons.contact),
                onPressed: () {
                  launchUrl(Uri.parse("https://bio.link/synaptic"));
                },
              ),
              IconButton(
                icon: Icon(FIcons.instagram),
                onPressed: () {
                  launchUrl(Uri.parse("https://www.instagram.com/itsKryxen"));
                },
              ),
            ],
          ),
        ],
      ),
    );
  }
}
