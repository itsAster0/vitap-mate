import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/vtop/route/vtop_proxy.dart';
import 'package:vitapmate/vtop/route/vtop_udp.dart';
import 'package:vitapmate/features/server/widgets/server_format.dart';
import 'package:vitapmate/features/server/widgets/server_sign_in_tile.dart';
import 'package:vitapmate/features/server/widgets/udp_mode_tile.dart';
import 'package:vitapmate/features/server/widgets/vtop_mode_picker.dart';

/// The path a request takes, then the VTOP route with its facts, and the
/// UDP transport under the server-side fetch route.
class VtopRouteSection extends ConsumerWidget {
  const VtopRouteSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(vtopModeProvider);
    final udp = ref.watch(vtopUdpProvider);
    final serverSignIn =
        ref
            .watch(settingsProvider)
            .value
            ?.getBool(bridgeServerSignInSettingKey) ??
        false;
    return Column(
      crossAxisAlignment: .stretch,
      children: [
        _RoutePath(switch (mode) {
          .off => 'Phone ─HTTPS→ VTOP',
          .compress => 'Phone ─HTTPS→ Edge /vtop/ → VTOP',
          .server when udp => 'Phone ─UDP · Noise→ Gateway → Engine → VTOP',
          .server => 'Phone ─HTTPS→ Edge → Engine → VTOP',
        }),
        const SizedBox(height: Space.sm),
        const VtopModePicker(),
        ServerFacts(switch (mode) {
          .off => const [
            ('Signs in', 'phone'),
            ('Data', 'full HTML over HTTPS'),
            ('Server', 'not used'),
          ],
          .compress => const [
            ('Signs in', 'phone'),
            ('Data', 'Brotli through the /vtop/ proxy'),
            ('Cookies', 'stay on the phone'),
          ],
          .server => [
            if (serverSignIn)
              ('Signs in', 'server, with the saved password and OTP')
            else
              ('Signs in', 'engine; password per sign-in, not stored'),
            ('If down', 'phone fetches directly'),
          ],
        }),
        if (mode == .server) ...[
          const SizedBox(height: Space.sm),
          const ServerSignInTile(),
          const SizedBox(height: Space.sm),
          const UdpModeTile(),
        ],
      ],
    );
  }
}

/// One line: where a request goes, hop by hop.
class _RoutePath extends StatelessWidget {
  const _RoutePath(this.path);

  final String path;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: colors.muted,
        borderRadius: .circular(Radii.sm + 2),
      ),
      child: Padding(
        padding: const .symmetric(horizontal: Space.md, vertical: Space.sm),
        child: Text(
          path,
          style: context.theme.typography.body.sm.copyWith(
            color: colors.foreground,
            fontWeight: .w500,
          ),
        ),
      ),
    );
  }
}
