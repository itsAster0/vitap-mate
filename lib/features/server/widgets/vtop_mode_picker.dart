import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/route/vtop_proxy.dart';
import 'package:vitapmate/features/server/server_account.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/server/widgets/expandable_picker.dart';

/// The VTOP mode. The data figures are measured: one sign-in with OTP and a
/// full refresh, 733 KB on the phone alone, 117 KB compressed, 29 KB
/// fetched on the server.
class VtopModePicker extends ConsumerWidget {
  const VtopModePicker({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    return ExpandablePicker<VtopServerMode>(
      value: ref.watch(vtopModeProvider),
      onSelect: (mode) async {
        await setVtopMode(ref, mode);
        // This phone now signs in through the server: apps need not wait
        // for it first.
        final choice = signInChoice(ref.read(serverAccountProvider).value);
        if (mode == .server && (choice == .wait10 || choice == .wait20)) {
          await ref.read(serverAccountProvider.notifier).setSignIn(.server);
        }
      },
      options: [
        PickerOption(
          value: .off,
          title: 'Direct (HTTPS to VTOP)',
          summary: '~730 KB per refresh',
          detail:
              'reqwest on the phone signs in and fetches full HTML from vtop.vitap.ac.in.',
          icon: FLucideIcons.smartphone,
          tone: colors.app.accentTone,
        ),
        PickerOption(
          value: .compress,
          title: 'Compressing proxy (/vtop/)',
          summary: '~120 KB · Brotli',
          detail:
              'Same requests from the phone, through the edge\'s /vtop/ proxy; responses Brotli-compressed.',
          icon: FLucideIcons.zap,
          tone: colors.app.lab,
        ),
        PickerOption(
          value: .server,
          title: 'Server-side fetch (engine)',
          summary: '~30 KB · packed JSON',
          detail:
              'The engine signs in and parses; the phone gets dictionary-packed JSON only.',
          icon: FLucideIcons.server,
          tone: colors.app.success,
        ),
      ],
    );
  }
}
