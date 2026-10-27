import 'dart:async';

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
import 'package:vitapmate/features/settings/presentation/widgets/email_otp_widgets.dart';

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

/// What agents may do with outings, as the switch and dialogs say it.
const outingPermissionTitle = 'Allow outing applications';
const outingPermissionDetail =
    'AI agents using this key can apply for and cancel your outings. They show you each request and need your yes before sending it.';

/// The new key's name, and whether it may act on outings (off unless
/// switched on here).
Future<({String label, bool outings})?> askKeyLabel(BuildContext context) {
  final controller = TextEditingController(text: 'My laptop');
  final outings = ValueNotifier(false);
  return showFDialog<({String label, bool outings})>(
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
          const SizedBox(height: Space.md),
          ValueListenableBuilder(
            valueListenable: outings,
            builder: (context, on, _) => FSwitch(
              label: const Text(outingPermissionTitle),
              description: const Text('Off unless you need it.'),
              value: on,
              onChange: (value) => outings.value = value,
            ),
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
            Navigator.of(context).pop((
              label: label.isEmpty ? 'Access key' : label,
              outings: outings.value,
            ));
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

/// Where a key is being set up, in [showAccessKey].
enum _Target { extension, agent }

/// How an AI agent is given the key.
enum _AgentSetup { claudeCode, header, urlOnly }

/// The key, then how to use it: the browser extension, or an AI agent
/// (Claude Code's command, a URL and header, or a URL with the key in it).
/// A sheet from the bottom, three quarters of the screen tall, so long
/// values have the whole width.
Future<void> showAccessKey(
  BuildContext context,
  BridgeKey key, {
  required bool fresh,
}) {
  return showFSheet<void>(
    context: context,
    side: FLayout.btt,
    mainAxisMaxRatio: _sheetRatio,
    builder: (context) => _KeySheet(bridgeKey: key, fresh: fresh),
  );
}

const _sheetRatio = 3 / 4;

class _KeySheet extends StatelessWidget {
  const _KeySheet({required this.bridgeKey, required this.fresh});

  final BridgeKey bridgeKey;
  final bool fresh;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Container(
      height: MediaQuery.sizeOf(context).height * _sheetRatio,
      decoration: BoxDecoration(
        color: colors.background,
        borderRadius: const BorderRadius.vertical(top: Radius.circular(10)),
      ),
      child: SafeArea(
        top: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(Space.lg, Space.md, Space.lg, 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const SheetHandle(),
              const SizedBox(height: Space.md),
              Row(
                children: [
                  Expanded(
                    child: Text(
                      fresh ? 'Your key is ready' : 'Access key',
                      style: typography.body.lg.copyWith(
                        fontWeight: FontWeight.w700,
                        color: colors.foreground,
                      ),
                    ),
                  ),
                  if (fresh)
                    ToneBadge(
                      label: 'NEW',
                      tone: colors.app.success,
                      icon: FLucideIcons.check,
                    ),
                ],
              ),
              const SizedBox(height: Space.md),
              Expanded(
                child: SingleChildScrollView(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      _KeyGuide(bridgeKey: bridgeKey, fresh: fresh),
                      const SizedBox(height: Space.lg),
                      FButton(
                        onPress: () => Navigator.of(context).pop(),
                        child: const Text('Done'),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _KeyGuide extends HookWidget {
  const _KeyGuide({required this.bridgeKey, required this.fresh});

  final BridgeKey bridgeKey;
  final bool fresh;

  @override
  Widget build(BuildContext context) {
    final target = useState(_Target.extension);
    final setup = useState(_AgentSetup.claudeCode);
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    final muted = typography.body.sm.copyWith(color: colors.mutedForeground);

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        _CopyBlock(label: 'Access key', value: bridgeKey.key, primary: true),
        const SizedBox(height: Space.sm),
        Text(
          fresh
              ? 'Treat it like a password. To see it again later, long-press it in the list on this phone.'
              : 'Treat it like a password: anyone with it can read your VTOP data.',
          style: muted,
        ),
        const SizedBox(height: Space.lg),
        Text(
          'Where will you use it?',
          style: typography.body.sm.copyWith(fontWeight: FontWeight.w600),
        ),
        const SizedBox(height: Space.sm),
        Segmented<_Target>(
          segments: const [
            (_Target.extension, 'Extension'),
            (_Target.agent, 'AI agent'),
          ],
          value: target.value,
          onChanged: (value) => target.value = value,
        ),
        const SizedBox(height: Space.md),
        AnimatedSize(
          duration: Motion.medium,
          curve: Curves.easeOutCubic,
          alignment: Alignment.topCenter,
          child: switch (target.value) {
            _Target.extension => const _Steps([
              'Copy the key above.',
              'Open the VITAP Mate extension and paste it in.',
            ]),
            _Target.agent => _AgentGuide(
              bridgeKey: bridgeKey,
              setup: setup.value,
              onSetup: (value) => setup.value = value,
            ),
          },
        ),
      ],
    );
  }
}

class _AgentGuide extends StatelessWidget {
  const _AgentGuide({
    required this.bridgeKey,
    required this.setup,
    required this.onSetup,
  });

  final BridgeKey bridgeKey;
  final _AgentSetup setup;
  final ValueChanged<_AgentSetup> onSetup;

  @override
  Widget build(BuildContext context) {
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final (option, title, detail) in const [
          (_AgentSetup.claudeCode, 'Claude Code', 'One command in a terminal'),
          (_AgentSetup.header, 'Other agents', 'A server URL and a header'),
          (_AgentSetup.urlOnly, 'URL only', 'For agents that only take a link'),
        ])
          _Option(
            title: title,
            detail: detail,
            selected: setup == option,
            onPress: () => onSetup(option),
          ),
        const SizedBox(height: Space.md),
        ...switch (setup) {
          _AgentSetup.claudeCode => [
            const _Steps(['Run this in a terminal. It adds VTOP as a tool.']),
            const SizedBox(height: Space.sm),
            _CopyBlock(
              label: 'Command',
              value: bridgeKey.claudeCodeCommand,
              wrap: true,
            ),
          ],
          _AgentSetup.header => [
            const _Steps([
              'Add a remote MCP server with this URL.',
              'Add this header to it.',
            ]),
            const SizedBox(height: Space.sm),
            _CopyBlock(label: 'MCP URL', value: bridgeKey.mcpUrl),
            const SizedBox(height: Space.sm),
            _CopyBlock(label: 'Header', value: bridgeKey.authHeader),
          ],
          _AgentSetup.urlOnly => [
            const _Steps([
              'For an agent that only asks for a URL: add this as its MCP server.',
            ]),
            const SizedBox(height: Space.sm),
            _CopyBlock(
              label: 'MCP URL with the key',
              value: bridgeKey.agentUrl,
            ),
            const SizedBox(height: Space.sm),
            GuideCallout(
              icon: FLucideIcons.triangleAlert,
              tone: context.theme.colors.app.warning,
              child: const Text(
                'The key is part of this link, so keep the link as secret as the key. Pick Other agents instead when yours takes a header.',
              ),
            ),
          ],
        },
      ],
    );
  }
}

/// One choice in a list, with a tick when it is the one picked.
class _Option extends StatelessWidget {
  const _Option({
    required this.title,
    required this.detail,
    required this.selected,
    required this.onPress,
  });

  final String title;
  final String detail;
  final bool selected;
  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final typography = context.theme.typography;
    return Padding(
      padding: const EdgeInsets.only(bottom: Space.xs),
      child: FTappable(
        selected: selected,
        semanticsLabel: '$title: $detail',
        onPress: () {
          if (!selected) HapticFeedback.selectionClick();
          onPress();
        },
        child: AnimatedContainer(
          duration: Motion.fast,
          padding: const EdgeInsets.symmetric(
            horizontal: Space.md,
            vertical: Space.sm,
          ),
          decoration: BoxDecoration(
            color: selected ? colors.background : colors.secondary,
            borderRadius: BorderRadius.circular(Radii.sm + 2),
            border: Border.all(
              color: selected ? colors.foreground : colors.border,
            ),
          ),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      style: typography.body.sm.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    Text(
                      detail,
                      style: typography.body.xs.copyWith(
                        color: colors.mutedForeground,
                      ),
                    ),
                  ],
                ),
              ),
              Icon(
                selected ? FLucideIcons.circleCheck : FLucideIcons.circle,
                size: 18,
                color: selected ? colors.foreground : colors.mutedForeground,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Numbered steps in the dialog's quiet text.
class _Steps extends StatelessWidget {
  const _Steps(this.steps);

  final List<String> steps;

  @override
  Widget build(BuildContext context) {
    final colors = context.theme.colors;
    final style = context.theme.typography.body.sm;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: Space.xs,
      children: [
        for (final (i, step) in steps.indexed)
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (steps.length > 1)
                SizedBox(
                  width: 20,
                  child: Text(
                    '${i + 1}.',
                    style: style.copyWith(color: colors.mutedForeground),
                  ),
                ),
              Expanded(child: Text(step, style: style)),
            ],
          ),
      ],
    );
  }
}

/// A value in a monospace box; tapping anywhere on it copies it, and the
/// icon turns into a tick for a moment. [primary] is the key itself, set
/// apart with the accent; [wrap] shows all of a long value.
class _CopyBlock extends HookWidget {
  const _CopyBlock({
    required this.label,
    required this.value,
    this.primary = false,
    this.wrap = false,
  });

  final String label;
  final String value;
  final bool primary;
  final bool wrap;

  @override
  Widget build(BuildContext context) {
    final copied = useState(false);
    useEffect(() {
      if (!copied.value) return null;
      final timer = Timer(const Duration(milliseconds: 1600), () {
        copied.value = false;
      });
      return timer.cancel;
    }, [copied.value]);

    final colors = context.theme.colors;
    final app = colors.app;
    final typography = context.theme.typography;
    final tone = copied.value ? app.success : app.accentTone;

    Future<void> copy() async {
      await Clipboard.setData(ClipboardData(text: value));
      HapticFeedback.lightImpact();
      copied.value = true;
    }

    return FTappable(
      semanticsLabel: 'Copy $label',
      onPress: copy,
      child: AnimatedContainer(
        duration: Motion.fast,
        padding: const EdgeInsets.fromLTRB(
          Space.md,
          Space.sm,
          Space.sm,
          Space.sm,
        ),
        decoration: BoxDecoration(
          color: primary ? tone.subtle : colors.secondary,
          borderRadius: BorderRadius.circular(Radii.sm + 2),
          border: Border.all(
            color: copied.value ? app.success.base : colors.border,
          ),
        ),
        child: Row(
          children: [
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    label,
                    style: typography.body.xs.copyWith(
                      color: colors.mutedForeground,
                    ),
                  ),
                  const SizedBox(height: 2),
                  Text(
                    value,
                    maxLines: wrap ? null : 1,
                    overflow: wrap ? null : TextOverflow.ellipsis,
                    style: typography.body.sm.copyWith(
                      fontFamily: 'monospace',
                      fontWeight: primary ? FontWeight.w600 : null,
                      color: primary ? tone.onSubtle : colors.foreground,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(width: Space.sm),
            AnimatedSwitcher(
              duration: Motion.fast,
              child: Row(
                key: ValueKey(copied.value),
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(
                    copied.value ? FLucideIcons.check : FLucideIcons.copy,
                    size: 16,
                    color: copied.value
                        ? app.success.base
                        : colors.mutedForeground,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    copied.value ? 'Copied' : 'Copy',
                    style: typography.body.xs.copyWith(
                      fontWeight: FontWeight.w600,
                      color: copied.value
                          ? app.success.base
                          : colors.mutedForeground,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
