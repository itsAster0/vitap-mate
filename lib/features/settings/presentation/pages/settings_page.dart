import 'dart:async';
import 'dart:developer' show log;
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/features/timetable/timetable.dart';
import 'package:vitapmate/features/attendance/attendance.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/app/theme_provider.dart';
import 'package:vitapmate/app/router/paths.dart';
import 'package:vitapmate/vtop/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/app/feature_flags.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/platform/background/controller.dart';
import 'package:vitapmate/platform/background/sync.dart';
import 'package:vitapmate/features/calendar/calendar.dart';
import 'package:vitapmate/features/exams/exams.dart';
import 'package:vitapmate/features/marks/marks.dart';
import 'package:vitapmate/features/settings/presentation/pages/user_management.dart';
import 'package:vitapmate/features/settings/presentation/widgets/mess_timings_sheet.dart';
import 'package:vitapmate/features/docs/presentation/providers/mess_timings_provider.dart';
import 'package:vitapmate/vtop/semesters.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/platform/update/update_check.dart';

class SettingsPage extends HookConsumerWidget {
  const SettingsPage({super.key});

  void _invalidateVtopDataProviders(WidgetRef ref) {
    ref.invalidate(attendanceProvider);
    ref.invalidate(examScheduleProvider);
    ref.invalidate(academicCalendarProvider);
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
    final isGmailOtpReady = useState(false);
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
    final updateSnapshot = useFuture(useMemoized(checkForAppUpdate, const []));
    final update = updateSnapshot.data;
    // Only a confirmed update is worth a tile; hide it when there is none or
    // when the check could not tell (offline, GitHub answered oddly, …).
    final hasUpdate = update?.available == true;

    Future<void> refreshEmailOtpReady() async {
      try {
        final flags = await ref.read(featureFlagsControllerProvider.future);
        final enabled = await flags.isEnabled('2fa-email');
        isEmailOtpFeatureEnabled.value = enabled;
        if (!enabled) {
          isEmailOtpReady.value = false;
          return;
        }
        ref
          ..invalidate(emailOtpReadyProvider)
          ..invalidate(anyEmailOtpReadyProvider);
        isGmailOtpReady.value = await ref.read(emailOtpReadyProvider.future);
        isEmailOtpReady.value = await ref.read(anyEmailOtpReadyProvider.future);
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
    final classesLeftUntil = ref.watch(classesLeftUntilProvider);

    final colors = context.theme.colors;
    Future<void> openGmailSetup() async {
      await context.pushNamed(Paths.gmailOtpSetup);
      await refreshEmailOtpReady();
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(
        Space.sm,
        Space.sm,
        Space.sm,
        Space.lg,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const UserBox(),
          SectionHeader(title: 'VTOP'),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              FTile(
                prefix: _IconTile(
                  icon: FLucideIcons.cloudDownload,
                  tone: colors.app.accentTone,
                ),
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
              if (isEmailOtpFeatureEnabled.value) ...[
                FTile(
                  prefix: _IconTile(
                    icon: FLucideIcons.mail,
                    tone: colors.app.warning,
                  ),
                  title: const Text('Email OTP'),
                  subtitle: isEmailOtpReady.value == true
                      ? Text(
                          'Set up · tap to manage',
                          style: TextStyle(color: colors.app.success.onSubtle),
                        )
                      : const Text('Auto-fill the VTOP sign-in OTP'),
                  suffix: isEmailOtpBusy.value
                      ? const FCircularProgress.pinwheel()
                      : isEmailOtpReady.value == true
                      ? const Icon(FLucideIcons.chevronRight)
                      : FButton(
                          variant: FButtonVariant.outline,
                          size: FButtonSizeVariant.sm,
                          mainAxisSize: MainAxisSize.min,
                          onPress: openGmailSetup,
                          child: const Text('Set up'),
                        ),
                  onPress: isEmailOtpBusy.value ? null : openGmailSetup,
                ),
                if (isGmailOtpReady.value)
                  FTile(
                    prefix: _IconTile(
                      icon: FLucideIcons.trash2,
                      tone: colors.app.warning,
                    ),
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
              ],
            ],
          ),
          SectionHeader(title: 'Privacy'),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              FTile(
                prefix: _IconTile(
                  icon: FLucideIcons.shieldCheck,
                  tone: colors.app.success,
                ),
                title: const Text('On-device Mode'),
                subtitle: const Text('Only talk to VTOP; no server features'),
                suffix: FSwitch(
                  value: ref.watch(onDeviceModeProvider),
                  onChange: (value) => _setOnDeviceMode(context, ref, value),
                ),
              ),
              FTile(
                prefix: _IconTile(
                  icon: FLucideIcons.server,
                  tone: colors.app.accentTone,
                ),
                title: const Text('Server'),
                subtitle: const Text('VTOP data, sign-in and connected apps'),
                suffix: const Icon(FLucideIcons.chevronRight),
                onPress: () => GoRouter.of(context).pushNamed(Paths.server),
              ),
            ],
          ),
          SectionHeader(title: 'Display'),
          _AppearancePicker(
            value: ref.watch(themeProvider),
            onChanged: (mode) =>
                ref.read(themeProvider.notifier).setThemeMode(mode),
            // Hidden: long-press toggles developer tools.
            onLongPress: () =>
                showDebugFeatures.value = !showDebugFeatures.value,
          ),
          const SizedBox(height: Space.sm + 2),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              FSelectMenuTile(
                prefix: _IconTile(
                  icon: FLucideIcons.calendarClock,
                  tone: colors.app.lab,
                ),
                title: FTappable(child: const Text('Classes Left')),
                subtitle: Text(switch (classesLeftUntil) {
                  ClassesLeftUntil.semesterEnd => 'Count to the semester end',
                  ClassesLeftUntil.nextExam => 'Count to the next exam',
                }),
                selectControl: FMultiValueControl.managedRadio(
                  initial: classesLeftUntil,
                  onChange: (value) {
                    if (value.isNotEmpty) {
                      setClassesLeftUntil(ref, value.first);
                    }
                  },
                ),
                menu: const [
                  FSelectTile(
                    title: Text('Semester end'),
                    subtitle: Text('Classes until the FAT'),
                    value: ClassesLeftUntil.semesterEnd,
                  ),
                  FSelectTile(
                    title: Text('Next exam'),
                    subtitle: Text('CAT-I → CAT-II → FAT'),
                    value: ClassesLeftUntil.nextExam,
                  ),
                ],
              ),
              FTile(
                prefix: _IconTile(icon: FLucideIcons.eye, tone: colors.app.lab),
                title: const Text('Show Skip Counts'),
                subtitle: const Text('How many classes you can skip or need'),
                suffix: FSwitch(
                  value: ref.watch(showSkipAdviceProvider),
                  onChange: (value) => setShowSkipAdvice(ref, value),
                ),
              ),
              FTile(
                prefix: _IconTile(
                  icon: FLucideIcons.utensils,
                  tone: colors.app.lab,
                ),
                title: const Text('Mess Timings'),
                subtitle: Text(
                  messTimingsSummary(ref.watch(messTimingsProvider)),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                suffix: Icon(FLucideIcons.chevronRight),
                onPress: () => showMessTimingsSheet(context),
              ),
              FTile(
                prefix: _IconTile(
                  icon: FLucideIcons.refreshCw,
                  tone: colors.app.lab,
                ),
                title: const Text('Refresh Button'),
                subtitle: const Text('Floating button that reloads the screen'),
                suffix: FSwitch(
                  value: ref.watch(refreshButtonProvider),
                  onChange: (value) => setRefreshButton(ref, value),
                ),
              ),
            ],
          ),
          SectionHeader(title: 'Sync'),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              FTile(
                prefix: _IconTile(
                  icon: FLucideIcons.refreshCcw,
                  tone: colors.app.success,
                ),
                title: const Text('Auto Refresh'),
                suffix: FSwitch(
                  value: ref.watch(autoRefreshProvider),
                  onChange: (value) {
                    setautoRefresh(ref, value);
                  },
                ),
              ),
              FSelectMenuTile(
                prefix: _IconTile(
                  icon: FLucideIcons.folderSync,
                  tone: colors.app.success,
                ),
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
              FTile(
                prefix: _IconTile(
                  icon: FLucideIcons.calendarPlus,
                  tone: colors.app.success,
                ),
                title: const Text('Calendar Sync'),
                subtitle: const Text('Add your classes to the phone calendar'),
                suffix: Icon(FLucideIcons.chevronRight),
                onPress: () {
                  GoRouter.of(context).pushNamed(Paths.calendarSync);
                },
              ),
              FTile(
                prefix: _IconTile(
                  icon: FLucideIcons.bell,
                  tone: colors.app.success,
                ),
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
              children: [
                FTile(
                  prefix: const Icon(FLucideIcons.bug),
                  title: const Text('Debug'),
                  subtitle: const Text('Activity, diagnostics, logs and tools'),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () => GoRouter.of(context).pushNamed(Paths.debug),
                ),
              ],
            ),
          Padding(
            padding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
            child: Text(
              'Classes left and skip counts are a best-effort estimate from '
              'the VTOP academic calendar and your timetable. Holidays '
              'announced later, cancelled or extra classes can change them.',
              textAlign: TextAlign.center,
              style: context.theme.typography.body.xs.copyWith(
                color: context.theme.colors.mutedForeground,
              ),
            ),
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
          if (hasUpdate && update != null) ...[
            const SizedBox(height: Space.lg),
            FTileGroup(
              children: [
                FTile(
                  prefix: _IconTile(
                    icon: FLucideIcons.download,
                    tone: colors.app.accentTone,
                  ),
                  title: const Text('Update available'),
                  subtitle: Text(
                    update.source == UpdateSource.github
                        ? 'Nightly ${update.latest} is available on GitHub'
                        : 'An update is available on the Play Store',
                  ),
                  suffix: const Icon(FLucideIcons.chevronRight),
                  onPress: () async {
                    try {
                      await installAppUpdate(update);
                    } catch (error, stackTrace) {
                      log(
                        'App update failed',
                        name: 'settings.update',
                        error: error,
                        stackTrace: stackTrace,
                      );
                      if (context.mounted) {
                        disCommonToast(context, error);
                      }
                    }
                  },
                ),
              ],
            ),
          ],
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

/// A row icon in a small tinted square; the tint groups related settings.
class _IconTile extends StatelessWidget {
  const _IconTile({required this.icon, required this.tone});

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

/// Theme choice as three small previews of the app in light, dark and
/// system (half and half).
class _AppearancePicker extends StatelessWidget {
  const _AppearancePicker({
    required this.value,
    required this.onChanged,
    required this.onLongPress,
  });

  final ThemeMode value;
  final ValueChanged<ThemeMode> onChanged;
  final VoidCallback onLongPress;

  static const _light = (
    Color(0xFFF4F4F5),
    Color(0xFFFFFFFF),
    Color(0xFFD4D4D8),
  );
  static const _dark = (
    Color(0xFF0A0A0A),
    Color(0xFF1F1F22),
    Color(0xFF3F3F46),
  );

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Surface(
      padding: const EdgeInsets.fromLTRB(
        Space.md,
        Space.md,
        Space.md,
        Space.md,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          GestureDetector(
            onLongPress: onLongPress,
            child: Row(
              children: [
                _IconTile(icon: FLucideIcons.sunMoon, tone: colors.app.lab),
                const SizedBox(width: Space.md),
                Text(
                  'Appearance',
                  style: typography.body.md.copyWith(
                    fontWeight: FontWeight.w500,
                    color: colors.foreground,
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: Space.md),
          Row(
            children: [
              for (final (i, (mode, label)) in const [
                (ThemeMode.light, 'Light'),
                (ThemeMode.dark, 'Dark'),
                (ThemeMode.system, 'System'),
              ].indexed) ...[
                if (i > 0) const SizedBox(width: Space.sm + 2),
                Expanded(
                  child: PressScale(
                    scale: 0.96,
                    semanticsLabel: '$label theme',
                    onPress: () => onChanged(mode),
                    child: Column(
                      children: [
                        AnimatedContainer(
                          duration: Motion.medium,
                          height: 64,
                          padding: const EdgeInsets.all(2),
                          decoration: BoxDecoration(
                            borderRadius: BorderRadius.circular(Radii.md),
                            border: Border.all(
                              color: value == mode
                                  ? colors.app.accent
                                  : colors.border,
                              width: value == mode ? 2 : 1,
                            ),
                          ),
                          child: ClipRRect(
                            borderRadius: BorderRadius.circular(Radii.md - 3),
                            child: switch (mode) {
                              ThemeMode.light => const _MiniApp(
                                palette: _light,
                              ),
                              ThemeMode.dark => const _MiniApp(palette: _dark),
                              ThemeMode.system => const Row(
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: [
                                  Expanded(child: _MiniApp(palette: _light)),
                                  Expanded(child: _MiniApp(palette: _dark)),
                                ],
                              ),
                            },
                          ),
                        ),
                        const SizedBox(height: Space.xs + 2),
                        Text(
                          label,
                          style: typography.body.xs.copyWith(
                            fontWeight: value == mode
                                ? FontWeight.w600
                                : FontWeight.w500,
                            color: value == mode
                                ? colors.foreground
                                : colors.mutedForeground,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ],
            ],
          ),
        ],
      ),
    );
  }
}

/// A tiny sketch of the app: background, a card and two text lines.
class _MiniApp extends StatelessWidget {
  const _MiniApp({required this.palette});

  final (Color, Color, Color) palette;

  @override
  Widget build(BuildContext context) {
    final (bg, card, line) = palette;
    return ColoredBox(
      color: bg,
      child: Padding(
        padding: const EdgeInsets.all(6),
        child: Container(
          decoration: BoxDecoration(
            color: card,
            borderRadius: BorderRadius.circular(4),
          ),
          padding: const EdgeInsets.all(5),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              FractionallySizedBox(
                widthFactor: 0.7,
                child: Container(
                  height: 4,
                  decoration: BoxDecoration(
                    color: line,
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
              const SizedBox(height: 4),
              FractionallySizedBox(
                widthFactor: 0.45,
                child: Container(
                  height: 4,
                  decoration: BoxDecoration(
                    color: line.withValues(alpha: 0.6),
                    borderRadius: BorderRadius.circular(2),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Turning on-device mode on asks first: it lists what stops working, and
/// what stays on the server until removed there.
Future<void> _setOnDeviceMode(
  BuildContext context,
  WidgetRef ref,
  bool value,
) async {
  if (!value) {
    await setOnDeviceMode(ref, false);
    return;
  }
  final prefs = await ref.read(settingsProvider.future);
  final linked = prefs.getBool(bridgeLinkedSettingKey) ?? false;
  if (!context.mounted) return;
  final confirmed = await confirm(
    context,
    title: 'Turn On On-device Mode?',
    action: 'Turn On',
    actionFirst: true,
    body:
        ('The app will only talk to VTOP (and Google, if you read OTPs from '
        'Gmail). These stop working until you turn it off: the browser '
        'extension, AI agents (MCP), OTP mail forwarding, server sign-in '
        'and a server data source.'
        '${linked ? '\n\nYour account on the VITAP Mate server, with any '
                  'saved password and access keys, stays there. To remove '
                  'it, delete it in Connected apps first.' : ''}'),
  );
  if (!confirmed) return;
  await setOnDeviceMode(ref, true);
  if (context.mounted) {
    dispToast(
      context,
      'On-device mode is on',
      'Vitap Mate now only talks to VTOP.',
    );
  }
}
