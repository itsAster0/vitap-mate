import 'package:flutter/material.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/on_device_mode.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';

const _serverNeededTitle = 'Needs the VITAP Mate server';

String _serverNeededMessage(String feature) =>
    'On-device mode is on, so the app only talks to VTOP. $feature uses '
    'the VITAP Mate server. Turn off on-device mode to use it.';

/// Shows [child] normally; in on-device mode, a note that [feature] needs
/// the server and a button that turns the mode off (then [child] appears).
class ServerFeatureGate extends ConsumerWidget {
  const ServerFeatureGate({
    super.key,
    required this.feature,
    required this.child,
  });

  /// What the page is, as it reads at the start of a sentence.
  final String feature;
  final Widget child;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    if (!ref.watch(onDeviceModeProvider)) return child;
    return Center(
      child: SingleChildScrollView(
        child: EmptyState(
          icon: FLucideIcons.shieldCheck,
          title: _serverNeededTitle,
          message: _serverNeededMessage(feature),
          action: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            spacing: Space.sm,
            children: [
              FButton(
                onPress: () => setOnDeviceMode(ref, false),
                child: const Text('Turn off on-device mode'),
              ),
              FButton(
                variant: FButtonVariant.outline,
                onPress: () => context.canPop() ? context.pop() : null,
                child: const Text('Go back'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// For entry points that open a dialog rather than a page: true when the
/// server may be used, else explains why not and offers to turn on-device
/// mode off (true if the user did).
Future<bool> ensureServerAllowed(
  BuildContext context,
  WidgetRef ref, {
  required String feature,
}) async {
  if (!ref.read(onDeviceModeProvider)) return true;
  final turnOff = await showAdaptiveDialog<bool>(
    context: context,
    barrierDismissible: true,
    builder: (dialogContext) => AppDialog(
      title: const Text(_serverNeededTitle),
      body: Text(_serverNeededMessage(feature)),
      actions: [
        FButton(
          onPress: () => Navigator.of(dialogContext).pop(true),
          child: const Text('Turn off on-device mode'),
        ),
        FButton(
          variant: FButtonVariant.outline,
          onPress: () => Navigator.of(dialogContext).pop(false),
          child: const Text('Cancel'),
        ),
      ],
    ),
  );
  if (turnOff != true) return false;
  await setOnDeviceMode(ref, false);
  return true;
}
