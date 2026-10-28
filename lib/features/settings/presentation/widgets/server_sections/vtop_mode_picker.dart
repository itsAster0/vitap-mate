import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/route/vtop_proxy.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/expandable_picker.dart';

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
      onSelect: (mode) => setVtopMode(ref, mode),
      options: [
        PickerOption(
          value: VtopServerMode.off,
          title: 'Phone only',
          summary: 'Most data',
          detail:
              'The phone signs in and fetches full pages, straight from VTOP.',
          icon: FLucideIcons.smartphone,
          tone: colors.app.accentTone,
        ),
        PickerOption(
          value: VtopServerMode.compress,
          title: 'Compress through the server',
          summary: 'About 6× less data',
          detail:
              'The phone still signs in and fetches; VTOP\'s pages reach it compressed.',
          icon: FLucideIcons.zap,
          tone: colors.app.lab,
        ),
        PickerOption(
          value: VtopServerMode.server,
          title: 'Fetch on the server',
          summary: 'About 25× less data',
          detail:
              'The server signs in and sends only what the app shows. The OTP is still asked on the phone.',
          icon: FLucideIcons.server,
          tone: colors.app.success,
        ),
      ],
    );
  }
}
