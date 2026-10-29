import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/route/vtop_udp.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/server/widgets/icon_box.dart';
import 'package:vitapmate/features/server/widgets/server_format.dart';

/// The UDP transport switch, under the server-side fetch route. Hidden in
/// builds that know no gateway.
class UdpModeTile extends ConsumerWidget {
  const UdpModeTile({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (!udpModeAvailable) return const SizedBox.shrink();
    final on = ref.watch(vtopUdpProvider);
    return Column(
      crossAxisAlignment: .stretch,
      children: [
        FTileGroup(
          children: [
            FTile(
              prefix: ServerIconBox(
                icon: FLucideIcons.radioTower,
                tone: context.theme.colors.app.lab,
              ),
              title: const Text('UDP transport'),
              subtitle: const Text('Noise IK + FEC over UDP'),
              suffix: FSwitch(
                value: on,
                onChange: (value) => setVtopUdp(ref, value),
              ),
            ),
          ],
        ),
        if (on) const ServerFacts([('Built for', '1–8 kbit/s, 10–20% loss')]),
      ],
    );
  }
}
