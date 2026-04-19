import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/features/background/controller.dart';
import 'package:vitapmate/features/settings/presentation/pages/user_management.dart';

class SettingsPage extends HookConsumerWidget {
  const SettingsPage({super.key});

  String _cookieEditorJsonFromHeader(String cookieHeader) {
    const domain = 'vtop.vitap.ac.in';
    final parts = cookieHeader.split(';');
    final cookies = <Map<String, dynamic>>[];

    for (var i = 0; i < parts.length; i++) {
      final part = parts[i].trim();
      if (part.isEmpty) continue;
      final eq = part.indexOf('=');
      if (eq <= 0) continue;
      final name = part.substring(0, eq).trim();
      final value = part.substring(eq + 1).trim();
      if (name.isEmpty) continue;

      cookies.add({
        'domain': domain,
        'hostOnly': true,
        'httpOnly': false,
        'name': name,
        'path': '/',
        'sameSite': 'unspecified',
        'secure': true,
        'session': true,
        'storeId': '0',
        'value': value,
        'id': i + 1,
      });
    }

    return const JsonEncoder.withIndent('  ').convert(cookies);
  }

  Future<void> _copySavedCookies(BuildContext context, WidgetRef ref) async {
    try {
      final user = await ref.read(vtopUserProvider.future);
      final username = user.username;
      if (username == null || username.isEmpty) {
        if (context.mounted) {
          dispToast(context, "No Account", "Sign in first to copy cookies.");
        }
        return;
      }

      final stored = await loadStoredVtopSession(username);
      final cookies = stored?.snapshot.cookies?.trim() ?? '';
      if (cookies.isEmpty) {
        if (context.mounted) {
          dispToast(
            context,
            "No Saved Cookies",
            "No saved session cookies found. Refresh data once and try again.",
          );
        }
        return;
      }

      final cookieEditorJson = _cookieEditorJsonFromHeader(cookies);
      if (cookieEditorJson == '[]') {
        if (context.mounted) {
          dispToast(
            context,
            "Invalid Cookie Data",
            "Saved cookie header could not be converted for Cookie-Editor.",
          );
        }
        return;
      }

      await Clipboard.setData(ClipboardData(text: cookieEditorJson));
      if (context.mounted) {
        dispToast(
          context,
          "Copied",
          "Cookie-Editor JSON copied to clipboard.",
        );
      }
    } catch (_) {
      if (context.mounted) {
        dispToast(context, "Failed", "Could not copy cookies right now.");
      }
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final show15 = useState(false);
    final field15e = useTextEditingController();
final showDebugFeastures = useState(false);
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
            if(showDebugFeastures.value)  FTile(
                prefix: const Icon(FIcons.copy),
                title: const Text('Copy Saved Cookies'),
                suffix: const Icon(FIcons.chevronRight),
                onPress: () => _copySavedCookies(context, ref),
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
                onLongPress: () => showDebugFeastures.value = !showDebugFeastures.value,
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
           if(showDebugFeastures.value)   FTile(
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
