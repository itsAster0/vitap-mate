import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/app/widgets/edit_number.dart';
import 'package:vitapmate/features/server/server.dart';
import 'package:vitapmate/features/settings/widgets/icon_tile.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/vtop/route/connection_profile.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';

/// How [profile] is named in Settings.
String connectionProfileTitle(ConnectionProfile profile) => switch (profile) {
  .phoneOnly => 'Direct',
  .compress => 'Balanced',
  .server => 'Data saver',
  .custom => 'Custom',
};

/// Settings → Connection: on-device mode, one profile for how the app
/// reaches VTOP, and Advanced (collapsed) with the settings a profile sets,
/// to change by hand. Data figures: one sign-in with OTP and a full
/// refresh, measured (733 KB direct, 117 KB compressed, 29 KB on the
/// server).
class ConnectionPage extends HookConsumerWidget {
  const ConnectionPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final profile = ref.watch(connectionProfileProvider);
    final onDevice = ref.watch(onDeviceModeProvider);
    final advanced = useState(false);

    FTile choice(
      ConnectionProfile value,
      String subtitle,
      IconData icon,
      Tone tone,
    ) => FTile(
      prefix: IconTile(icon: icon, tone: tone),
      title: Text(connectionProfileTitle(value)),
      subtitle: Text(subtitle),
      suffix: profile == value
          ? Icon(FLucideIcons.check, color: colors.primary)
          : null,
      onPress: () => _choose(context, ref, value),
    );

    return Container(
      color: colors.background,
      child: SingleChildScrollView(
        padding: const .fromLTRB(Space.sm, Space.sm, Space.sm, Space.lg),
        child: Column(
          crossAxisAlignment: .stretch,
          children: [
            FTileGroup(
              children: [
                FTile(
                  prefix: IconTile(
                    icon: FLucideIcons.shieldCheck,
                    tone: colors.app.success,
                  ),
                  title: const Text('On-device Mode'),
                  subtitle: const Text('Only talk to VTOP; no server features'),
                  suffix: FSwitch(
                    value: onDevice,
                    onChange: (value) => _setOnDeviceMode(context, ref, value),
                  ),
                ),
              ],
            ),
            if (!onDevice) ...[
              const SectionHeader(title: 'Profile'),
              FTileGroup(
                divider: .indented,
                children: [
                  choice(
                    .phoneOnly,
                    'Phone ↔ VTOP · ~730 KB per refresh',
                    FLucideIcons.smartphone,
                    colors.app.accentTone,
                  ),
                  choice(
                    .compress,
                    'Compressed by the server · ~120 KB, 6× less',
                    FLucideIcons.zap,
                    colors.app.lab,
                  ),
                  choice(
                    .server,
                    '~30 KB, 25× less · best on weak networks',
                    FLucideIcons.server,
                    colors.app.success,
                  ),
                ],
              ),
              if (profile == .custom)
                Padding(
                  padding: const .fromLTRB(4, Space.sm, 4, 0),
                  child: Text(
                    'Custom: changed under Advanced',
                    style: context.theme.typography.body.sm.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
                ),
              FTappable(
                onPress: () => advanced.value = !advanced.value,
                child: SectionHeader(
                  title: 'Advanced',
                  trailing: Icon(
                    advanced.value
                        ? FLucideIcons.chevronUp
                        : FLucideIcons.chevronDown,
                    size: 18,
                    color: colors.mutedForeground,
                  ),
                ),
              ),
              if (advanced.value) ...[
                const VtopRouteSection(),
                const SectionHeader(title: 'Timeouts'),
                FTileGroup(
                  divider: .indented,
                  children: [
                    for (final tunable in tunables)
                      FTile(
                        prefix: IconTile(
                          icon: FLucideIcons.hourglass,
                          tone: colors.app.accentTone,
                        ),
                        title: Text(tunable.title),
                        subtitle: Text(
                          '${ref.watch(tunableProvider(tunable))}${tunable.unit}',
                        ),
                        suffix: const Icon(FLucideIcons.chevronRight),
                        onPress: () => editNumber(
                          context,
                          title: tunable.title,
                          prompt: 'Default: ${tunable.fallback}${tunable.unit}',
                          current: ref.read(tunableProvider(tunable)),
                          onSave: (value) => setTunable(ref, tunable, value),
                        ),
                      ),
                  ],
                ),
              ],
            ],
          ],
        ),
      ),
    );
  }
}

/// Applies [profile]; Data saver offers server sign-in when it is off.
Future<void> _choose(
  BuildContext context,
  WidgetRef ref,
  ConnectionProfile profile,
) async {
  await applyConnectionProfile(ref, profile);
  if (profile != .server || !context.mounted) return;
  try {
    await offerServerSignIn(context, ref);
  } catch (error) {
    if (context.mounted) disCommonToast(context, error);
  }
}

/// Turning on-device mode on asks first: it lists what stops working, and
/// what stays on the server until removed there.
Future<void> _setOnDeviceMode(
  BuildContext context,
  WidgetRef ref,
  bool value,
) async {
  if (!value) return setOnDeviceMode(ref, false);
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
