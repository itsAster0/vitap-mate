import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/vtop_udp.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/icon_box.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/server_format.dart';

/// The experimental UDP mode switch, under "Fetch on the server". Hidden in
/// builds that know no gateway.
class UdpModeTile extends ConsumerWidget {
  const UdpModeTile({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (!udpModeAvailable) return const SizedBox.shrink();
    final on = ref.watch(vtopUdpProvider);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        FTileGroup(
          children: [
            FTile(
              prefix: ServerIconBox(
                icon: FLucideIcons.radioTower,
                tone: context.theme.colors.app.lab,
              ),
              title: const Text('UDP mode'),
              subtitle: const Text('Experimental, for slow networks'),
              suffix: FSwitch(
                value: on,
                onChange: (value) => setVtopUdp(ref, value),
              ),
            ),
          ],
        ),
        if (on)
          const ServerNote(
            'Pages come over UDP through a separate gateway, which keeps your session for six hours. Sign-in still uses HTTPS, and so does everything on networks that block UDP.',
          ),
      ],
    );
  }
}
