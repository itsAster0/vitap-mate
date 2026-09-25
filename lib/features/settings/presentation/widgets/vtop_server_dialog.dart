import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/vtop_backend/remote_vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/vtop_server_settings.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';

String _host(VtopServerSettings settings) {
  final host = Uri.tryParse(settings.url.trim())?.host;
  return host == null || host.isEmpty ? settings.url.trim() : host;
}

/// Subtitle for the Data Source tile.
String vtopDataSourceLabel(VtopServerSettings settings) {
  if (settings.isEnabled) return 'Server · ${_host(settings)}';
  if (settings.hasUrl) return 'On device · server saved';
  return 'On device';
}

Future<void> showVtopServerDialog(BuildContext context, WidgetRef ref) {
  final current = ref.read(vtopServerSettingsProvider);
  return showAdaptiveDialog<void>(
    context: context,
    barrierDismissible: true,
    builder: (dialogContext) => _VtopServerDialog(
      initial: current,
      onSave: (value) => setVtopServerSettings(ref, value),
      parentContext: context,
    ),
  );
}

class _VtopServerDialog extends HookWidget {
  const _VtopServerDialog({
    required this.initial,
    required this.onSave,
    required this.parentContext,
  });

  final VtopServerSettings initial;
  final Future<void> Function(VtopServerSettings) onSave;
  final BuildContext parentContext;

  @override
  Widget build(BuildContext context) {
    final useServer = useState(initial.isEnabled);
    final url = useTextEditingController(text: initial.url);
    final apiKey = useTextEditingController(text: initial.apiKey);
    final busy = useState(false);
    final check = useState<VtopServerCheck?>(null);
    final error = useState<String?>(null);
    final typography = context.theme.typography;
    final colors = context.theme.colors;

    VtopServerSettings current() => VtopServerSettings(
      url: url.text,
      apiKey: apiKey.text,
      enabled: useServer.value,
    );

    /// The settings to check against a server, or null (with a message)
    /// when the URL is missing or malformed.
    VtopServerSettings? serverToCheck() {
      final urlError = url.text.trim().isEmpty
          ? 'Enter the server URL.'
          : VtopServerSettings.validateUrl(url.text);
      error.value = urlError;
      return urlError == null ? current() : null;
    }

    Future<void> test() async {
      final value = serverToCheck();
      if (value == null) return;
      busy.value = true;
      check.value = await checkVtopServer(value);
      busy.value = false;
    }

    Future<void> save() async {
      if (useServer.value) {
        final value = serverToCheck();
        if (value == null) return;
        busy.value = true;
        final result = await checkVtopServer(value);
        busy.value = false;
        check.value = result;
        if (!result.isOk) return;
      } else {
        // Off: keep whatever details were typed, but don't require them.
        error.value = null;
      }
      await onSave(current());
      if (!context.mounted) return;
      Navigator.of(context).pop();
      if (parentContext.mounted) {
        dispToast(
          parentContext,
          'Saved',
          useServer.value
              ? 'VTOP data and login now go through ${_host(current())}.'
              : 'VTOP data is fetched on this device.',
        );
      }
    }

    final status = check.value;
    final message =
        error.value ??
        (status == null
            ? null
            : status.isOk
            ? 'Connected · vtop-server v${status.version}'
            : status.error);
    final messageIsError =
        error.value != null || (status != null && !status.isOk);

    return AppDialog(
      title: const Text('Data Source'),
      body: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          FTileGroup(
            children: [
              FTile(
                prefix: Icon(
                  useServer.value
                      ? FLucideIcons.server
                      : FLucideIcons.smartphone,
                ),
                title: const Text('Use a vtop-server'),
                subtitle: Text(
                  useServer.value ? 'Through your server' : 'On this phone',
                ),
                suffix: FSwitch(
                  value: useServer.value,
                  onChange: busy.value
                      ? null
                      : (value) {
                          useServer.value = value;
                          error.value = null;
                          check.value = null;
                        },
                ),
                onPress: busy.value
                    ? null
                    : () {
                        useServer.value = !useServer.value;
                        error.value = null;
                        check.value = null;
                      },
              ),
            ],
          ),
          AnimatedSize(
            duration: const Duration(milliseconds: 220),
            curve: Curves.easeOutCubic,
            alignment: Alignment.topCenter,
            child: !useServer.value
                ? (initial.hasUrl
                      ? Padding(
                          padding: const EdgeInsets.only(top: 10),
                          child: Text(
                            'Saved: ${_host(initial)} · switch on to use it again',
                            style: typography.body.xs.copyWith(
                              color: colors.mutedForeground,
                            ),
                          ),
                        )
                      : const SizedBox(width: double.infinity))
                : Padding(
                    padding: const EdgeInsets.only(top: 14),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      spacing: 12,
                      children: [
                        FTextField(
                          control: FTextFieldControl.managed(controller: url),
                          label: const Text('Server URL'),
                          hint: 'https://vtop.example.com',
                          keyboardType: TextInputType.url,
                          autocorrect: false,
                        ),
                        FTextField.password(
                          control: FTextFieldControl.managed(
                            controller: apiKey,
                          ),
                          label: const Text('API key'),
                          description: const Text('Optional'),
                          autocorrect: false,
                        ),
                        Text(
                          'Sign-in also goes through this server, so it sees your VTOP password. Use one you run or trust.',
                          style: typography.body.xs.copyWith(
                            color: colors.mutedForeground,
                          ),
                        ),
                      ],
                    ),
                  ),
          ),
          if (message != null)
            Padding(
              padding: const EdgeInsets.only(top: 12),
              child: Row(
                spacing: 6,
                children: [
                  Icon(
                    messageIsError
                        ? FLucideIcons.circleAlert
                        : FLucideIcons.circleCheck,
                    size: 14,
                    color: messageIsError ? colors.destructive : colors.primary,
                  ),
                  Expanded(
                    child: Text(
                      message,
                      style: typography.body.sm.copyWith(
                        color: messageIsError
                            ? colors.destructive
                            : colors.primary,
                      ),
                    ),
                  ),
                ],
              ),
            ),
        ],
      ),
      actions: [
        if (useServer.value)
          FButton(
            variant: FButtonVariant.outline,
            onPress: busy.value ? null : test,
            child: busy.value
                ? const FCircularProgress.pinwheel()
                : const Text('Test connection'),
          ),
        FButton(onPress: busy.value ? null : save, child: const Text('Save')),
      ],
    );
  }
}
