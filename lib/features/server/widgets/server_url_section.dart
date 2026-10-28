import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/vtop/route/server_url.dart';
import 'package:vitapmate/app/app_urls.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:vitapmate/vtop/edge/server_switch.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';
import 'package:vitapmate/ui/app_dialog.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/features/server/widgets/icon_box.dart';

const serverUrlFormatError =
    'Use https://host (plain http only on this computer or a private network), without a path.';
const serverUnreachableError = 'No VITAP Mate server answers at that address.';

/// The server URL in use, and changing or resetting it.
class ServerUrlSection extends HookConsumerWidget {
  const ServerUrlSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final url = ref.watch(serverUrlProvider);
    final isDefault = ref.read(serverUrlProvider.notifier).isDefault;
    final client = ref.watch(appHttpClientProvider);
    final reachable = useFuture(
      useMemoized(() => probeServer(url, client), [url]),
    );
    final colors = context.theme.colors;
    final (statusText, statusColor) = switch (reachable) {
      AsyncSnapshot(connectionState: .waiting) => (
        'Checking…',
        colors.mutedForeground,
      ),
      AsyncSnapshot(data: true) => ('Ready', colors.app.success.base),
      _ => ('Unavailable', colors.app.danger.base),
    };

    Future<void> confirmSwitch(String? next) async {
      if (!await confirm(
        context,
        title: 'Switch servers?',
        body:
            'This phone leaves ${Uri.parse(url).host}: its link, access keys and OTP forwarding address stay there. It registers with ${Uri.parse(next ?? defaultServerUrl).host} instead; link it and set up forwarding again there.',
        action: 'Switch',
        direction: .horizontal,
      )) {
        return;
      }
      try {
        await switchServer(ref, next);
      } catch (error) {
        if (context.mounted) dispToast(context, 'Could not switch', '$error');
      }
    }

    Future<void> edit() async {
      final next = await showFDialog<String>(
        context: context,
        useRootNavigator: true,
        builder: (context, style, animation) =>
            _UrlDialog(animation: animation, initial: url, client: client),
      );
      if (next != null && next != url) await confirmSwitch(next);
    }

    return Column(
      crossAxisAlignment: .stretch,
      children: [
        SectionHeader(
          title: 'Server',
          trailing: Text(
            statusText,
            style: context.theme.typography.body.xs.copyWith(
              color: statusColor,
            ),
          ),
        ),
        FTileGroup(
          divider: .indented,
          children: [
            FTile(
              prefix: ServerIconBox(
                icon: FLucideIcons.server,
                tone: colors.app.accentTone,
              ),
              title: Text(url),
              subtitle: Text(isDefault ? 'Default' : 'Set on this phone'),
              suffix: const Icon(FLucideIcons.pencil),
              onPress: edit,
            ),
            if (!isDefault)
              FTile(
                prefix: ServerIconBox(
                  icon: FLucideIcons.rotateCcw,
                  tone: colors.app.warning,
                ),
                title: const Text('Use the default server'),
                subtitle: Text(defaultServerUrl),
                onPress: () => confirmSwitch(null),
              ),
          ],
        ),
      ],
    );
  }
}

class _UrlDialog extends HookWidget {
  const _UrlDialog({
    required this.animation,
    required this.initial,
    required this.client,
  });

  final Animation<double> animation;
  final String initial;
  final http.Client client;

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController(text: initial);
    final error = useState<String?>(null);
    final checking = useState(false);

    Future<void> save() async {
      final normalized = normalizeServerUrl(controller.text);
      if (normalized == null) {
        error.value = serverUrlFormatError;
        return;
      }
      checking.value = true;
      error.value = null;
      final ok = await probeServer(normalized, client);
      checking.value = false;
      if (!ok) {
        error.value = serverUnreachableError;
      } else if (context.mounted) {
        Navigator.of(context).pop(normalized);
      }
    }

    return AppDialog(
      animation: animation,
      direction: .horizontal,
      title: const Text('Server URL'),
      body: Column(
        mainAxisSize: .min,
        crossAxisAlignment: .stretch,
        children: [
          const Text('The VITAP Mate server this phone uses.'),
          const SizedBox(height: Space.sm),
          FTextField(
            control: FTextFieldControl.managed(controller: controller),
            keyboardType: .url,
            autocorrect: false,
          ),
          if (error.value case final message?) ...[
            const SizedBox(height: Space.xs),
            Text(
              message,
              style: TextStyle(color: context.theme.colors.destructive),
            ),
          ],
        ],
      ),
      actions: [
        FButton(
          variant: .outline,
          onPress: () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FButton(
          onPress: checking.value ? null : save,
          child: const Text('Save'),
        ),
      ],
    );
  }
}
