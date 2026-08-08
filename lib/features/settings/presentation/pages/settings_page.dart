import 'dart:async';
import 'dart:developer' show log;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
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
import 'package:vitapmate/features/settings/presentation/providers/semester_id_provider.dart';
import 'package:vitapmate/features/timetable/presentation/providers/timetable_provider.dart';

const _appTrack = String.fromEnvironment(
  'APP_TRACK',
  defaultValue: 'production',
);

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

  Future<void> _openEmailOtpSetupDialog(
    BuildContext context,
    WidgetRef ref,
  ) async {
    try {
      await ref.read(vClientProvider.notifier).ensureLogin();
      final client = await ref.read(vClientProvider.future);
      final expectedUsername = client.username.trim();
      if (expectedUsername.isEmpty) {
        if (context.mounted) {
          dispToast(
            context,
            "Setup Failed",
            "Could not resolve VTOP username from current client.",
          );
        }
        return;
      }
      final oauth = ref.read(googleEmailOtpAuthServiceProvider);
      if (!context.mounted) return;

      await showAdaptiveDialog(
        context: context,
        builder: (dialogContext) {
          return HookBuilder(
            builder: (dialogContext) {
              final verifiedEmail = useState<String?>(null);
              final stepOneMessage = useState('Step 1 pending');
              final stepTwoMessage = useState('Step 2 pending');
              final stepOneBusy = useState(false);
              final stepTwoBusy = useState(false);

              return AppDialog(
                title: const Text('Email OTP Setup'),
                body: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'Complete both steps to enable OTP autofetch from Gmail.',
                    ),
                    const SizedBox(height: 12),
                    Text(
                      '1) Verify email',
                      style: dialogContext.theme.typography.body.sm.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    Text(
                      stepOneMessage.value,
                      style: dialogContext.theme.typography.body.sm,
                    ),
                    const SizedBox(height: 8),
                    FButton(
                      onPress: stepOneBusy.value || verifiedEmail.value != null
                          ? null
                          : () async {
                              stepOneBusy.value = true;
                              final result = await oauth.setupIdentityStep(
                                expectedUsername: expectedUsername,
                              );
                              stepOneMessage.value = result.message;
                              verifiedEmail.value = result.success
                                  ? result.email
                                  : null;
                              stepOneBusy.value = false;
                            },
                      child: stepOneBusy.value
                          ? const FCircularProgress.pinwheel()
                          : Text(
                              verifiedEmail.value == null
                                  ? 'Run Step 1'
                                  : 'Step 1 Done',
                            ),
                    ),
                    const SizedBox(height: 14),
                    Text(
                      '2) Get tokens for email read',
                      style: dialogContext.theme.typography.body.sm.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    Text(
                      stepTwoMessage.value,
                      style: dialogContext.theme.typography.body.sm,
                    ),
                    if (verifiedEmail.value != null) ...[
                      const SizedBox(height: 4),
                      Text(
                        'Verified email: ${verifiedEmail.value}',
                        style: dialogContext.theme.typography.body.xs.copyWith(
                          color: dialogContext.theme.colors.mutedForeground,
                        ),
                      ),
                    ],
                    const SizedBox(height: 8),
                    FButton(
                      onPress: stepTwoBusy.value || verifiedEmail.value == null
                          ? null
                          : () async {
                              stepTwoBusy.value = true;
                              final result = await oauth.setupGmailTokenStep(
                                email: verifiedEmail.value!,
                              );
                              stepTwoMessage.value = result.message;
                              stepTwoBusy.value = false;
                              if (result.success && dialogContext.mounted) {
                                Navigator.of(dialogContext).pop();
                              }
                            },
                      child: stepTwoBusy.value
                          ? const FCircularProgress.pinwheel()
                          : const Text('Run Step 2'),
                    ),
                  ],
                ),
                actions: [
                  FButton(
                    variant: FButtonVariant.outline,
                    onPress: () => Navigator.of(dialogContext).pop(),
                    child: const Text('Close'),
                  ),
                ],
              );
            },
          );
        },
      );
      ref.invalidate(emailOtpReadyProvider);
    } catch (error, stackTrace) {
      log(
        'Failed to open Email OTP setup dialog',
        name: 'settings.email_otp',
        error: error,
        stackTrace: stackTrace,
      );
      if (context.mounted) {
        dispToast(
          context,
          "Setup Failed",
          "Could not start setup. Please try again.",
        );
      }
    }
  }

  Future<void> _disconnectEmailOtpAutofetch(
    BuildContext context,
    WidgetRef ref,
  ) async {
    final confirm = await showAdaptiveDialog<bool>(
      context: context,
      builder: (dialogContext) => AppDialog(
        title: const Text('Disconnect Email OTP?'),
        body: const Text(
          'This removes the saved Gmail token. You can connect it again later.',
        ),
        actions: [
          FButton(
            variant: FButtonVariant.outline,
            onPress: () => Navigator.of(dialogContext).pop(false),
            child: const Text('Cancel'),
          ),
          FButton(
            variant: FButtonVariant.destructive,
            onPress: () => Navigator.of(dialogContext).pop(true),
            child: const Text('Disconnect'),
          ),
        ],
      ),
    );
    if (confirm != true) return;

    try {
      await ref.read(googleEmailOtpAuthServiceProvider).clearSession();
      ref.invalidate(emailOtpReadyProvider);
      if (context.mounted) {
        dispToast(
          context,
          "Disconnected",
          "Email OTP autofetch has been reset.",
        );
      }
    } catch (error, stackTrace) {
      log(
        'Failed to clear Email OTP session',
        name: 'settings.email_otp',
        error: error,
        stackTrace: stackTrace,
      );
      if (context.mounted) {
        dispToast(context, "Failed", "Could not clear Email OTP autofetch.");
      }
    }
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
        dispToast(context, "Updated", "All VTOP data is up to date.");
      } else {
        dispToast(
          context,
          "Partially Updated",
          "Some VTOP data could not be refreshed. Try again in a bit.",
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
    final showLowMaintenanceNotice = _appTrack == 'production';

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
      child: Column(
        spacing: 8,
        children: [
          if (showLowMaintenanceNotice)
            const Padding(
              padding: EdgeInsets.fromLTRB(12, 0, 12, 4),
              child: FAlert(
                variant: FAlertVariant.primary,
                title: Text('Low maintenance mode'),
                subtitle: Text(
                  'This app is in low maintenance mode and might not receive future updates.',
                ),
              ),
            ),
          const UserBox(),
          FTileGroup(
            divider: FItemDivider.indented,
            label: const Text('VTOP Data'),
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
                prefix: Icon(FLucideIcons.userCheck),
                title: const Text('B/W exams'),
                subtitle: const Text('Between exams attendance'),
                suffix: FSwitch(
                  value: ref.watch(btwExamsProvider),
                  onChange: (value) {
                    setbtwExam(ref, value);
                  },
                ),
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
          if (isEmailOtpFeatureEnabled.value)
            FTileGroup(
              divider: FItemDivider.indented,
              label: const Text('Email OTP'),
              children: [
                FTile(
                  prefix: const Icon(FLucideIcons.mail),
                  title: const Text('Gmail Autofetch'),
                  subtitle: Text(
                    isEmailOtpReady.value == true
                        ? 'Connected · tap to manage'
                        : 'Connect Gmail for automatic OTPs',
                    style: isEmailOtpReady.value == true
                        ? null
                        : TextStyle(color: context.theme.colors.destructive),
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
                          isEmailOtpBusy.value = true;
                          if (isEmailOtpReady.value == true) {
                            await _disconnectEmailOtpAutofetch(context, ref);
                          } else {
                            await _openEmailOtpSetupDialog(context, ref);
                          }
                          await refreshEmailOtpReady();
                          isEmailOtpBusy.value = false;
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
          FTileGroup(
            divider: FItemDivider.indented,
            label: const Text('Sync'),
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
          FTileGroup(
            divider: FItemDivider.indented,
            label: const Text('App Settings'),
            children: [
              FTile(
                prefix: Icon(FLucideIcons.moon),
                title: const Text('Dark Mode'),
                onLongPress: () {
                  showDebugFeatures.value = !showDebugFeatures.value;
                },
                suffix: FSwitch(
                  value: ref.watch(themeProvider) == ThemeMode.dark,
                  onChange: (value) {
                    ref.read(themeProvider.notifier).toggleTheme();
                  },
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
          if (showDebugFeatures.value)
            FTileGroup(
              divider: FItemDivider.indented,
              label: const Text('Developer Tools'),
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
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              IconButton(
                tooltip: 'Source code',
                icon: const Icon(Icons.code),
                onPressed: () {
                  launchUrl(
                    Uri.parse("https://github.com/itsKryxen/vitap-mate"),
                  );
                },
              ),
              IconButton(
                tooltip: 'Developer profile',
                icon: Icon(FLucideIcons.contact),
                onPressed: () {
                  launchUrl(Uri.parse("https://bio.link/synaptic"));
                },
              ),
              IconButton(
                tooltip: 'Instagram',
                icon: const Icon(Icons.camera_alt_outlined),
                onPressed: () {
                  launchUrl(Uri.parse("https://www.instagram.com/itsKryxen"));
                },
              ),
            ],
          ),
          if (appVersion != null)
            Padding(
              padding: const EdgeInsets.only(top: 4, bottom: 16),
              child: Text(
                appVersion,
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
