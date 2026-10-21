import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:local_auth/local_auth.dart';
import 'package:vitapmate/core/utils/edge/server_status.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';

/// A yes/no question with a destructive [action] button.
Future<bool> confirmAction(
  BuildContext context,
  String title,
  String body,
  String action,
) => confirm(
  context,
  title: title,
  body: body,
  action: action,
  destructive: true,
  direction: Axis.horizontal,
);

/// A busy flag, and [run] for one server action at a time: the status is
/// refreshed after it, and a toast titled `failure` shows if it throws.
({
  ValueNotifier<bool> busy,
  Future<void> Function(String failure, Future<void> Function() action) run,
})
useServerAction(BuildContext context, WidgetRef ref) {
  final busy = useState(false);
  Future<void> run(String failure, Future<void> Function() action) async {
    if (busy.value) return;
    busy.value = true;
    try {
      await action();
      ref.invalidate(serverStatusProvider);
    } catch (error) {
      if (context.mounted) dispToast(context, failure, '$error');
    } finally {
      busy.value = false;
    }
  }

  return (busy: busy, run: run);
}

Future<String?> askKeyLabel(BuildContext context) {
  final controller = TextEditingController(text: 'My laptop');
  return showFDialog<String>(
    context: context,
    useRootNavigator: true,
    builder: (context, style, animation) => AppDialog(
      animation: animation,
      direction: Axis.horizontal,
      title: const Text('Name this key'),
      body: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text('So you can tell your keys apart later.'),
          const SizedBox(height: Space.sm),
          FTextField(
            control: FTextFieldControl.managed(controller: controller),
          ),
        ],
      ),
      actions: [
        FButton(
          variant: FButtonVariant.outline,
          onPress: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FButton(
          onPress: () {
            final label = controller.text.trim();
            Navigator.of(context).pop(label.isEmpty ? 'Access key' : label);
          },
          child: const Text('Create'),
        ),
      ],
    ),
  );
}

/// Asks for the device's biometrics or screen lock, when it has either.
Future<bool> confirmKeyOwner() async {
  final auth = LocalAuthentication();
  try {
    if (!await auth.isDeviceSupported()) return true;
    return await auth.authenticate(
      localizedReason: 'Confirm it\'s you to show this access key',
    );
  } catch (_) {
    return false;
  }
}

Future<void> showAccessKey(
  BuildContext context,
  BridgeKey key, {
  required bool fresh,
}) {
  return showFDialog<void>(
    context: context,
    useRootNavigator: true,
    builder: (context, style, animation) => AppDialog(
      animation: animation,
      title: Text(fresh ? 'Copy your key' : 'Access key'),
      body: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            fresh
                ? 'Paste the key into the browser extension. For an AI agent, add the MCP URL and send the key in the Authorization header. Long-press it in the list to show it again on this phone.'
                : 'Paste the key into the browser extension. For an AI agent, add the MCP URL and send the key in the Authorization header.',
          ),
          const SizedBox(height: Space.md),
          _CopyRow(label: 'Access key', value: key.key),
          _CopyRow(label: 'MCP URL', value: key.mcpUrl),
          _CopyRow(label: 'MCP header', value: key.authHeader),
          _CopyRow(label: 'Claude Code', value: key.claudeCodeCommand),
        ],
      ),
      actions: [
        FButton(
          onPress: () => Navigator.of(context).pop(),
          child: const Text('Done'),
        ),
      ],
    ),
  );
}

class _CopyRow extends StatelessWidget {
  const _CopyRow({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final typography = context.theme.typography;
    return Padding(
      padding: const EdgeInsets.only(bottom: Space.sm),
      child: Row(
        children: [
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  label,
                  style: typography.body.xs.copyWith(
                    color: context.theme.colors.mutedForeground,
                  ),
                ),
                Text(
                  value,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: typography.body.sm.copyWith(fontFamily: 'monospace'),
                ),
              ],
            ),
          ),
          FButton(
            variant: FButtonVariant.outline,
            size: FButtonSizeVariant.sm,
            mainAxisSize: MainAxisSize.min,
            onPress: () async {
              await Clipboard.setData(ClipboardData(text: value));
              if (context.mounted) dispToast(context, 'Copied', label);
            },
            child: const Text('Copy'),
          ),
        ],
      ),
    );
  }
}
