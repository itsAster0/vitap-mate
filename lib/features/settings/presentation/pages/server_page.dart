import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/on_device_mode.dart';
import 'package:vitapmate/core/providers/server_url.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/vtop_proxy.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/edge/server_status.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/icon_box.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/server_format.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/status_card.dart';

/// Where the Server page lives; Connected apps' old route goes here too.
const serverPagePath = '/settings/server';

/// Linked phones one account may have; the edge's `MAX_PHONES`.
const maxPhones = 5;

/// The Server hub: a status card, then one row per area.
class ServerPage extends ConsumerWidget {
  const ServerPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final onDevice = ref.watch(onDeviceModeProvider);
    final colors = context.theme.colors;
    final prefs = ref.watch(settingsProvider).value;
    final linked = prefs?.getBool(bridgeLinkedSettingKey) ?? false;

    if (onDevice) {
      return ServerSubPage(
        title: 'Server',
        children: [
          const SizedBox(height: Space.md),
          const FAlert(
            title: Text('On-device Mode is on'),
            subtitle: Text(
              'The app talks only to VTOP. Turn it off to use the server.',
            ),
          ),
          const SizedBox(height: Space.sm),
          FButton(
            onPress: () => setOnDeviceMode(ref, false),
            child: const Text('Turn off On-device Mode'),
          ),
          const ServerNote(
            'Compressing VTOP, connected apps (browser extension, AI agents) and OTP forwarding all need it.',
          ),
        ],
      );
    }

    final status = ref.watch(serverStatusProvider).value;
    final url = ref.watch(serverUrlProvider);
    final isDefault = ref.read(serverUrlProvider.notifier).isDefault;

    FTile row(
      String title,
      IconData icon,
      String route, {
      String? subtitle,
      Tone? tone,
    }) => FTile(
      prefix: ServerIconBox(icon: icon, tone: tone ?? colors.app.accentTone),
      title: Text(title),
      subtitle: subtitle == null ? null : Text(subtitle),
      suffix: const Icon(FLucideIcons.chevronRight),
      onPress: () => context.pushNamed(route),
    );

    return ServerSubPage(
      title: 'Server',
      children: [
        const SizedBox(height: Space.md),
        ServerStatusCard(linked: linked),
        const SectionHeader(title: 'VTOP'),
        FTileGroup(
          children: [
            FTile(
              prefix: ServerIconBox(
                icon: FLucideIcons.zap,
                tone: colors.app.success,
              ),
              title: const Text('Compress through the server', maxLines: 2),
              subtitle: const Text(
                'About 10× less data. The server passes on your sign-in.',
                maxLines: 2,
              ),
              suffix: FSwitch(
                value: ref.watch(vtopProxyProvider),
                onChange: (on) => setVtopProxy(ref, on),
              ),
            ),
          ],
        ),
        const ServerNote(
          'The phone signs in and fetches either way; with this on, VTOP\'s pages reach it compressed through the server.',
        ),
        const SectionHeader(title: 'Apps and phones'),
        FTileGroup(
          divider: FItemDivider.indented,
          children: [
            row(
              'Connected apps',
              FLucideIcons.plug,
              Paths.serverApps,
              subtitle: status == null
                  ? 'Browser extension and AI agents'
                  : '${status.keys.length} ${status.keys.length == 1 ? 'key' : 'keys'}',
            ),
            row(
              'Linked phones',
              FLucideIcons.smartphone,
              Paths.serverPhones,
              subtitle: status == null
                  ? (linked ? 'This phone is linked' : 'Not linked')
                  : '${status.phones} of $maxPhones',
            ),
          ],
        ),
        const SectionHeader(title: 'Server'),
        FTileGroup(
          divider: FItemDivider.indented,
          children: [
            row(
              'Server address',
              FLucideIcons.server,
              Paths.serverAddress,
              subtitle: isDefault ? 'Default' : Uri.tryParse(url)?.host ?? url,
            ),
            row(
              'Your data on the server',
              FLucideIcons.shieldCheck,
              Paths.serverData,
              tone: colors.app.success,
            ),
          ],
        ),
      ],
    );
  }
}
