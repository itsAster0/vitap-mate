import 'dart:async';
import 'dart:developer' show log;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/features/attendance/presentation/providers/attendance_provider.dart';
import 'package:vitapmate/features/background/controller.dart';
import 'package:vitapmate/features/background/sync.dart';
import 'package:vitapmate/features/more/presentation/providers/exam_schedule.dart';
import 'package:vitapmate/features/more/presentation/providers/marks_provider.dart';
import 'package:vitapmate/features/settings/presentation/pages/user_management.dart';
import 'package:vitapmate/features/settings/presentation/widgets/vtop_server_dialog.dart';
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';

class SettingsPage extends HookConsumerWidget {
  const SettingsPage({super.key});

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

      final cookieEditorJson = cookieEditorJsonFromHeader(cookies);
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
        dispToast(context, "Copied", "Cookie-Editor JSON copied to clipboard.");
      }
    } catch (error, stackTrace) {
      log(
        'Failed to copy saved cookies',
        name: 'settings',
        error: error,
        stackTrace: stackTrace,
      );
      if (context.mounted) {
        dispToast(context, "Failed", "Could not copy cookies right now.");
      }
    }
  }

  Future<void> _copyFcmToken(BuildContext context) async {
    try {
      final token = await getFcmTokenForCopy();
      if (token == null || token.trim().isEmpty) {
        if (context.mounted) {
          dispToast(context, "No Token", "Could not get your Token right now.");
        }
        return;
      }

      await Clipboard.setData(ClipboardData(text: token));
      if (context.mounted) {
        dispToast(context, "Copied", "Token copied to clipboard.");
      }
    } catch (error, stackTrace) {
      log(
        'Failed to copy FCM token',
        name: 'settings.fcm',
        error: error,
        stackTrace: stackTrace,
      );
      if (context.mounted) {
        dispToast(context, "Failed", "Could not copy your Token right now.");
      }
    }
  }

  Future<void> _clearSavedCookies(BuildContext context, WidgetRef ref) async {
    try {
      final user = await ref.read(vtopUserProvider.future);
      final username = user.username;
      if (username == null || username.isEmpty) {
        if (context.mounted) {
          dispToast(context, "No Account", "Sign in first to clear cookies.");
        }
        return;
      }

      if (!context.mounted) return;
      final confirmed = await showAdaptiveDialog<bool>(
        context: context,
        barrierDismissible: true,
        builder: (_) => AppDialog(
          title: const Text('Clear Saved Cookies?'),
          body: const Text(
            'This removes the saved VTOP session cookies for your account. You may need to sign in again.',
          ),
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () =>
                  Navigator.of(context, rootNavigator: true).pop(false),
              child: const Text('Cancel'),
            ),
            FButton(
              variant: FButtonVariant.destructive,
              onPress: () =>
                  Navigator.of(context, rootNavigator: true).pop(true),
              child: const Text('Clear'),
            ),
          ],
        ),
      );

      if (confirmed != true) return;

      await clearStoredVtopSession(username);
      if (context.mounted) {
        dispToast(context, "Cleared", "Saved VTOP cookies were cleared.");
      }
    } catch (error, stackTrace) {
      log(
        'Failed to clear saved cookies',
        name: 'settings',
        error: error,
        stackTrace: stackTrace,
      );
      if (context.mounted) {
        dispToast(context, "Failed", "Could not clear cookies right now.");
      }
    }
  }

  Future<void> _openVtopSessionReuseTtlDialog(
    BuildContext context,
    WidgetRef ref,
  ) async {
    final currentTtl = ref.read(vtopSessionReuseTtlProvider);

    await showAdaptiveDialog<void>(
      context: context,
      barrierDismissible: true,
      builder: (dialogContext) {
        return HookBuilder(
          builder: (dialogContext) {
            final controller = useTextEditingController(
              text: currentTtl.inMinutes.toString(),
            );
            final errorText = useState<String?>(null);

            return AppDialog(
              title: const Text('VTOP Session Reuse'),
              body: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text('Enter how long saved cookies can be reused.'),
                  const SizedBox(height: 8),
                  FTextField(
                    control: FTextFieldControl.managed(controller: controller),
                    keyboardType: TextInputType.number,
                    inputFormatters: [FilteringTextInputFormatter.digitsOnly],
                  ),
                  if (errorText.value != null) ...[
                    const SizedBox(height: 8),
                    Text(
                      errorText.value!,
                      style: dialogContext.theme.typography.body.sm.copyWith(
                        color: dialogContext.theme.colors.destructive,
                      ),
                    ),
                  ],
                ],
              ),
              actions: [
                FButton(
                  variant: FButtonVariant.outline,
                  onPress: () => Navigator.of(dialogContext).pop(),
                  child: const Text('Cancel'),
                ),
                FButton(
                  onPress: () async {
                    final minutes = int.tryParse(controller.text.trim());
                    if (minutes == null || minutes <= 0) {
                      errorText.value = 'Enter minutes greater than 0.';
                      return;
                    }

                    await setVtopSessionReuseTtl(
                      ref,
                      Duration(minutes: minutes),
                    );
                    if (!dialogContext.mounted) return;
                    Navigator.of(dialogContext).pop();
                    if (context.mounted) {
                      dispToast(
                        context,
                        'Saved',
                        'VTOP sessions will be reused for $minutes minutes.',
                      );
                    }
                  },
                  child: const Text('Save'),
                ),
              ],
            );
          },
        );
      },
    );
  }

  Future<void> _testLatestInfoEmail(BuildContext context, WidgetRef ref) async {
    try {
      final latest = await ref
          .read(googleEmailOtpAuthServiceProvider)
          .fetchLatestInfoEmail(
            deleteAfterReading: ref.read(emailOtpDeleteAfterReadingProvider),
          );
      if (!context.mounted) return;

      await showAdaptiveDialog<void>(
        context: context,
        builder: (dialogContext) {
          if (latest == null) {
            return AppDialog(
              title: const Text('Latest OTP Email'),
              body: const Text('No emails from info1@vitap.ac.in were found.'),
              actions: [
                FButton(
                  onPress: () => Navigator.of(dialogContext).pop(),
                  child: const Text('Close'),
                ),
              ],
            );
          }

          final localTime = latest.receivedAt.toLocal().toString();
          return AppDialog(
            title: const Text('Latest OTP Email'),
            body: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Subject: ${latest.subject}'),
                const SizedBox(height: 6),
                Text('Received: $localTime'),
                const SizedBox(height: 6),
                Text('OTP: ${latest.otp ?? 'Not found'}'),
                const SizedBox(height: 12),
                Text(latest.snippet),
              ],
            ),
            actions: [
              FButton(
                onPress: () => Navigator.of(dialogContext).pop(),
                child: const Text('Close'),
              ),
            ],
          );
        },
      );
    } catch (error, stackTrace) {
      log(
        'Failed to fetch latest info1 email',
        name: 'settings.email_otp',
        error: error,
        stackTrace: stackTrace,
      );
      if (context.mounted) {
        dispToast(context, "Test Failed", "$error");
      }
    }
  }

  void _invalidateVtopDataProviders(WidgetRef ref) {
    ref.invalidate(attendanceProvider);
    ref.invalidate(examScheduleProvider);
    ref.invalidate(marksProvider);
    ref.invalidate(semesterIdProvider);
    ref.invalidate(timetableProvider);
  }

  Future<void> _refreshAllVtopData(BuildContext context, WidgetRef ref) async {
    final stopwatch = Stopwatch()..start();
    String took() =>
        'Took ${(stopwatch.elapsedMilliseconds / 1000).toStringAsFixed(1)} s.';
    try {
      final success = await syncVtopData(
        read: ref.read,
        task: 'manual_vtop_sync',
        force: true,
        promptForOtp: true,
        ignoreRecoverableErrors: false,
      );
      _invalidateVtopDataProviders(ref);
      if (!context.mounted) return;
      if (success) {
        dispToast(context, "Updated", "All VTOP data is up to date. ${took()}");
      } else {
        dispToast(
          context,
          "Partially Updated",
          "Some VTOP data could not be refreshed. Try again in a bit. ${took()}",
        );
      }
    } catch (error, stackTrace) {
      log(
        'Manual VTOP data refresh failed',
        name: 'settings.vtop_sync',
        error: error,
        stackTrace: stackTrace,
      );
      if (context.mounted) {
        disCommonToast(context, error);
      }
    }
  }

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final showDebugFeatures = useState(false);
    final isEmailOtpReady = useState<bool?>(null);
    final isEmailOtpBusy = useState(false);
    final isEmailOtpTestBusy = useState(false);
    final isEmailOtpFeatureEnabled = useState(false);
    final isVtopSyncing = useState(false);
    final packageInfoSnapshot = useFuture(
      useMemoized(PackageInfo.fromPlatform, const []),
    );
    final packageInfo = packageInfoSnapshot.data;
    final appVersion = packageInfo == null
        ? null
        : 'Version ${packageInfo.version} (${packageInfo.buildNumber})';

    Future<void> refreshEmailOtpReady() async {
      try {
        final flags = await ref.read(featureFlagsControllerProvider.future);
        final enabled = await flags.isEnabled('2fa-email');
        isEmailOtpFeatureEnabled.value = enabled;
        if (!enabled) {
          isEmailOtpReady.value = false;
          return;
        }
        isEmailOtpReady.value = await ref
            .read(googleEmailOtpAuthServiceProvider)
            .isReady();
      } catch (error, stackTrace) {
        log(
          'Failed to refresh Email OTP ready state',
          name: 'settings.email_otp',
          error: error,
          stackTrace: stackTrace,
        );
        isEmailOtpFeatureEnabled.value = false;
        isEmailOtpReady.value = false;
      }
    }

    useEffect(() {
      unawaited(refreshEmailOtpReady());
      return null;
    }, const []);
    final backgroundSync = [
      FSelectTile(title: Text("Disable"), value: Duration(hours: 0)),
      FSelectTile(title: Text("3 hours"), value: Duration(hours: 3)),
      FSelectTile(title: Text("6 hours"), value: Duration(hours: 6)),
      FSelectTile(title: Text("12 hours"), value: Duration(hours: 12)),
      FSelectTile(title: Text("24 hours"), value: Duration(hours: 24)),
    ];
    final initialValSync =
        ref.watch(backgroundSyncProvider).value?.freq ?? Duration(seconds: 0);
    final backgroundSyncLabel = initialValSync == Duration.zero
        ? 'Disabled'
        : 'Every ${initialValSync.inHours} hours';
    final initialVtopSessionReuseTtl = ref.watch(vtopSessionReuseTtlProvider);

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(0, Space.sm, 0, Space.lg),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const UserBox(),
          SectionHeader(title: 'VTOP Data'),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              FTile(
                prefix: Icon(FLucideIcons.calendarDays),
                title: const Text('Merge Labs'),
                subtitle: const Text('Combine consecutive lab slots'),
                suffix: FSwitch(
                  value: ref.watch(mergeTTProvider),
                  onChange: (value) {
                    setMergeTT(ref, value);
                  },
                ),
              ),
              FTile(
                prefix: const Icon(FLucideIcons.arrowDownUp),
                title: const Text('Data Source'),
                subtitle: Text(
                  vtopDataSourceLabel(ref.watch(vtopServerSettingsProvider)),
                ),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => showVtopServerDialog(context, ref),
              ),
              FTile(
                prefix: const Icon(FLucideIcons.cloudDownload),
                title: const Text('Update VTOP Data'),
                subtitle: const Text('Refresh all VTOP data now'),
                suffix: isVtopSyncing.value
                    ? const FCircularProgress.pinwheel()
                    : const Icon(FLucideIcons.chevronRight),
                onPress: isVtopSyncing.value
                    ? null
                    : () async {
                        isVtopSyncing.value = true;
                        await _refreshAllVtopData(context, ref);
                        if (context.mounted) {
                          isVtopSyncing.value = false;
                        }
                      },
              ),
            ],
          ),
          if (isEmailOtpFeatureEnabled.value) SectionHeader(title: 'Email OTP'),
          if (isEmailOtpFeatureEnabled.value)
            FTileGroup(
              divider: FItemDivider.indented,
              children: [
                FTile(
                  prefix: const Icon(FLucideIcons.mail),
                  title: const Text('Gmail Autofetch'),
                  subtitle: isEmailOtpReady.value == true
                      ? const Text('Connected · tap to manage')
                      : Align(
                          alignment: Alignment.centerLeft,
                          child: ToneBadge(
                            label: 'NOT CONNECTED',
                            tone: context.theme.colors.app.warning,
                          ),
                        ),
                  suffix: isEmailOtpBusy.value
                      ? const FCircularProgress.pinwheel()
                      : Icon(
                          isEmailOtpReady.value == true
                              ? FLucideIcons.chevronRight
                              : FLucideIcons.link,
                        ),
                  onPress: isEmailOtpBusy.value
                      ? null
                      : () async {
                          await context.pushNamed(Paths.gmailOtpSetup);
                          await refreshEmailOtpReady();
                        },
                ),
                if (isEmailOtpReady.value == true)
                  FTile(
                    prefix: const Icon(FLucideIcons.trash2),
                    title: const Text('Delete After Reading'),
                    subtitle: const Text(
                      'Move fetched OTP emails to Gmail Trash',
                    ),
                    suffix: FSwitch(
                      value: ref.watch(emailOtpDeleteAfterReadingProvider),
                      onChange: (value) {
                        setEmailOtpDeleteAfterReading(ref, value);
                      },
                    ),
                  ),
                if (isEmailOtpReady.value == true && showDebugFeatures.value)
                  FTile(
                    prefix: const Icon(FLucideIcons.mailCheck),
                    title: const Text('Test Latest OTP Email'),
                    subtitle: const Text('Fetch the latest VTOP OTP email'),
                    suffix: isEmailOtpTestBusy.value
                        ? const FCircularProgress.pinwheel()
                        : const Icon(FLucideIcons.chevronRight),
                    onPress: isEmailOtpTestBusy.value
                        ? null
                        : () async {
                            isEmailOtpTestBusy.value = true;
                            await _testLatestInfoEmail(context, ref);
                            isEmailOtpTestBusy.value = false;
                          },
                  ),
              ],
            ),
          SectionHeader(title: 'Sync'),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              FTile(
                prefix: Icon(FLucideIcons.refreshCcw),
                title: const Text('Auto Refresh'),
                suffix: FSwitch(
                  value: ref.watch(autoRefreshProvider),
                  onChange: (value) {
                    setautoRefresh(ref, value);
                  },
                ),
              ),
              FSelectMenuTile(
                prefix: Icon(FLucideIcons.folderSync),
                title: FTappable(child: Text('Background Sync')),
                subtitle: Text(backgroundSyncLabel),
                selectControl: FMultiValueControl.managedRadio(
                  initial: initialValSync,
                  onChange: (value) {
                    final selected = value.isEmpty ? null : value.first;
                    if (selected != null) {
                      ref
                          .read(backgroundSyncProvider.notifier)
                          .updateFreq(selected);
                    }
                  },
                ),
                menu: backgroundSync,
              ),
            ],
          ),
          SectionHeader(title: 'App Settings'),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              FTile(
                prefix: Icon(FLucideIcons.sunMoon),
                title: const Text('Appearance'),
                onLongPress: () {
                  showDebugFeatures.value = !showDebugFeatures.value;
                },
                suffix: SizedBox(
                  width: 204,
                  child: Segmented<ThemeMode>(
                    value: ref.watch(themeProvider),
                    onChanged: (mode) =>
                        ref.read(themeProvider.notifier).setThemeMode(mode),
                    segments: const [
                      (ThemeMode.light, 'Light'),
                      (ThemeMode.dark, 'Dark'),
                      (ThemeMode.system, 'System'),
                    ],
                  ),
                ),
              ),
              FTile(
                prefix: Icon(FLucideIcons.bell),
                title: const Text('Notifications'),
                subtitle: const Text('Manage class and exam reminders'),
                suffix: Icon(FLucideIcons.chevronRight),
                onPress: () {
                  GoRouter.of(context).pushNamed(Paths.notificationManagement);
                },
              ),
            ],
          ),
          if (showDebugFeatures.value) SectionHeader(title: 'Developer Tools'),
          if (showDebugFeatures.value)
            FTileGroup(
              divider: FItemDivider.indented,
              children: [
                FTile(
                  prefix: const Icon(FLucideIcons.timer),
                  title: const Text('VTOP Session Reuse'),
                  subtitle: Text(
                    'Reuse saved cookies for ${initialVtopSessionReuseTtl.inMinutes} minutes',
                  ),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () => _openVtopSessionReuseTtlDialog(context, ref),
                ),
                FTile(
                  prefix: const Icon(FLucideIcons.radio),
                  title: const Text('Copy Token'),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () => _copyFcmToken(context),
                ),
                FTile(
                  prefix: const Icon(FLucideIcons.copy),
                  title: const Text('Copy Saved Cookies'),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () => _copySavedCookies(context, ref),
                ),
                FTile(
                  prefix: const Icon(FLucideIcons.trash2),
                  title: const Text('Clear Saved Cookies'),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () => _clearSavedCookies(context, ref),
                ),
                FTile(
                  prefix: const Icon(Icons.receipt_long_outlined),
                  title: const Text('Logs'),
                  suffix: Icon(FLucideIcons.chevronRight),
                  onPress: () {
                    GoRouter.of(context).pushNamed(Paths.logs);
                  },
                ),
              ],
            ),
          Padding(
            padding: const EdgeInsets.only(top: 8),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                _FooterLink(
                  icon: Icons.code,
                  label: 'Source',
                  url: "https://github.com/itsKryxen/vitap-mate",
                ),
                _FooterLink(
                  icon: FLucideIcons.contact,
                  label: 'Developer',
                  url: "https://bio.link/synaptic",
                ),
                _FooterLink(
                  icon: Icons.camera_alt_outlined,
                  label: 'Instagram',
                  url: "https://www.instagram.com/itsKryxen",
                ),
              ],
            ),
          ),
          if (appVersion != null)
            Padding(
              padding: const EdgeInsets.only(top: 4, bottom: 16),
              child: Text(
                appVersion,
                textAlign: TextAlign.center,
                style: context.theme.typography.body.sm.copyWith(
                  color: context.theme.colors.mutedForeground,
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _FooterLink extends StatelessWidget {
  const _FooterLink({
    required this.icon,
    required this.label,
    required this.url,
  });

  final IconData icon;
  final String label;
  final String url;

  @override
  Widget build(BuildContext context) {
    final muted = context.theme.colors.mutedForeground;
    return Semantics(
      link: true,
      label: label,
      child: FTappable(
        onPress: () => launchUrl(Uri.parse(url)),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 10),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 20, color: muted),
              const SizedBox(height: 4),
              Text(
                label,
                style: context.theme.typography.body.xs.copyWith(color: muted),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
