import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/edge/server_status.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/features/settings/presentation/pages/server_page.dart';
import 'package:vitapmate/features/settings/presentation/providers/server_account.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/expandable_picker.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/icon_box.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/key_dialogs.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/server_format.dart';

enum _Offline { phone, wait10, wait20, server }

const _maxKeys = 10;

String _via(String via) => via == 'mcp' ? 'MCP' : 'Extension';

/// Access keys for the browser extension and AI agents (MCP).
class ServerAppsPage extends HookConsumerWidget {
  const ServerAppsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final account = ref.watch(serverAccountProvider);
    final status = ref.watch(serverStatusProvider).value;
    final (:busy, :run) = useServerAction(context, ref);
    final actions = ref.read(serverAccountProvider.notifier);
    final keys = account.value?.keys ?? const <BridgeKeyInfo>[];
    // What is in flight, so its own row shows it.
    final creating = useState(false);
    final revoking = useState<String?>(null);
    final pendingOffline = useState<_Offline?>(null);

    Future<void> create() async {
      final current = account.value;
      if (current != null &&
          !current.linkedHere &&
          current.phoneCount >= maxPhones &&
          !await confirmAction(
            context,
            'Link this phone?',
            'The account already has $maxPhones phones linked. Linking here unlinks the one used least recently.',
            'Link',
          )) {
        return;
      }
      if (!context.mounted) return;
      final label = await askKeyLabel(context);
      if (label == null) return;
      creating.value = true;
      try {
        await run('Could not create a key', () async {
          final key = await actions.createKey(label);
          creating.value = false;
          if (context.mounted) await showAccessKey(context, key, fresh: true);
        });
      } finally {
        if (context.mounted) creating.value = false;
      }
    }

    Future<void> showSaved(BridgeKeyInfo info) async {
      final saved = await SecureBridgeKeyStash().read(info.id);
      if (!context.mounted) return;
      if (saved == null) {
        dispToast(
          context,
          'Not saved on this phone',
          'Keys can be shown again only on the phone that created them. Create a new key to copy one.',
        );
        return;
      }
      if (!await confirmKeyOwner()) return;
      if (context.mounted) await showAccessKey(context, saved, fresh: false);
    }

    Future<void> revoke(BridgeKeyInfo key) async {
      if (!await confirmAction(
        context,
        'Revoke "${key.label}"?',
        'Apps using this key stop working right away.',
        'Revoke',
      )) {
        return;
      }
      revoking.value = key.id;
      try {
        await run('Could not revoke', () => actions.revokeKey(key.id));
      } finally {
        if (context.mounted) revoking.value = null;
      }
    }

    List<KeyUse> usesOf(String id) => [
      for (final key in status?.keys ?? const <KeyActivity>[])
        if (key.id == id) ...key.uses,
    ];

    // How offline access behaves, from the saved password and its settings.
    final saved = account.value?.savedCredentials ?? false;
    final settings = account.value?.settings ?? const BridgeSettings();
    final offline = !saved
        ? _Offline.phone
        : settings.alwaysUseVault
        ? _Offline.server
        : settings.phoneWaitSecs <= 10
        ? _Offline.wait10
        : _Offline.wait20;

    Future<void> saveOffline(_Offline mode) => run(
      'Could not change',
      () async {
        if (mode == _Offline.phone) {
          await actions.setServerSignIn(false);
          return;
        }
        // Seal the password first if it is not already (needs OTP forwarding).
        if (!(account.value?.savedCredentials ?? false)) {
          await actions.setServerSignIn(true);
        }
        await actions.updateSettings(
          BridgeSettings(
            alwaysUseVault: mode == _Offline.server,
            phoneWaitSecs: mode == _Offline.wait10 ? 10 : 20,
          ),
        );
      },
    );

    Future<void> chooseOffline(_Offline mode) async {
      pendingOffline.value = mode;
      try {
        await saveOffline(mode);
      } finally {
        if (context.mounted) pendingOffline.value = null;
      }
    }

    return ServerSubPage(
      title: 'Connected apps',
      children: [
        const ServerNote(
          'One access key works in the browser extension and in AI agents (MCP). The phone that created a key signs in for it when needed.',
        ),
        if (!account.hasValue && !account.hasError)
          const _AppsSkeleton()
        else if (!account.hasValue) ...[
          const SizedBox(height: Space.md),
          FAlert(
            variant: FAlertVariant.destructive,
            title: const Text('Could not load your account'),
            subtitle: Text('${account.error}'),
          ),
          const SizedBox(height: Space.sm),
          FButton(
            variant: FButtonVariant.outline,
            onPress: account.isLoading
                ? null
                : () => ref.invalidate(serverAccountProvider),
            prefix: account.isLoading
                ? const FCircularProgress.pinwheel()
                : const Icon(FLucideIcons.refreshCw),
            child: Text(account.isLoading ? 'Trying again…' : 'Try again'),
          ),
        ] else ...[
          if (status?.credentialsRejectedAt case final rejected?
              when status?.vaultSavedAt == null ||
                  rejected.isAfter(status!.vaultSavedAt!)) ...[
            const FAlert(
              variant: FAlertVariant.destructive,
              title: Text('VTOP rejected your saved password'),
              subtitle: Text('Pick a server option again to save a fresh one.'),
            ),
            const SizedBox(height: Space.sm),
          ],
          ExpandablePicker<_Offline>(
            label: 'When the phone is offline',
            value: offline,
            pending: pendingOffline.value,
            enabled: !busy.value,
            onSelect: chooseOffline,
            options: [
              PickerOption(
                value: _Offline.phone,
                title: 'Only this phone',
                summary: 'Nothing on the server',
                detail: 'Apps wait for this phone to sign in.',
                icon: FLucideIcons.smartphone,
                tone: colors.app.accentTone,
              ),
              PickerOption(
                value: _Offline.wait10,
                title: 'Wait 10s, then the server',
                summary: 'This phone first',
                detail: 'Give this phone 10 seconds, then the server signs in.',
                icon: FLucideIcons.timer,
                tone: colors.app.lab,
              ),
              PickerOption(
                value: _Offline.wait20,
                title: 'Wait 20s, then the server',
                summary: 'This phone first, longer',
                detail: 'Give this phone 20 seconds, then the server signs in.',
                icon: FLucideIcons.hourglass,
                tone: colors.app.lab,
              ),
              PickerOption(
                value: _Offline.server,
                title: 'Server signs in right away',
                summary: 'Fastest for apps',
                detail: 'This phone is not asked first.',
                icon: FLucideIcons.server,
                tone: colors.app.success,
              ),
            ],
          ),
          const ServerNote(
            'The server signing in needs your password sealed on it and OTP email forwarding set up.',
          ),
          SectionHeader(
            title: 'Access keys',
            trailing: Text('${keys.length}/$_maxKeys'),
          ),
          FTileGroup(
            divider: FItemDivider.indented,
            children: [
              for (final key in keys)
                FTile(
                  prefix: ServerIconBox(
                    icon: FLucideIcons.key,
                    tone: colors.app.accentTone,
                  ),
                  title: Text(key.label),
                  subtitle: Text(
                    revoking.value == key.id
                        ? 'Revoking…'
                        : [
                            key.id,
                            for (final use in usesOf(key.id).take(3))
                              '${_via(use.via)} ${ago(use.at)}',
                            if (usesOf(key.id).isEmpty)
                              key.lastUsedAt == null
                                  ? 'never used'
                                  : 'used ${ago(key.lastUsedAt!)}',
                          ].join(' · '),
                  ),
                  onLongPress: busy.value ? null : () => showSaved(key),
                  suffix: revoking.value == key.id
                      ? const Padding(
                          padding: EdgeInsets.all(Space.sm),
                          child: FCircularProgress.pinwheel(),
                        )
                      : FButton.icon(
                          variant: FButtonVariant.ghost,
                          onPress: busy.value ? null : () => revoke(key),
                          child: const Icon(FLucideIcons.trash2),
                        ),
                ),
              FTile(
                prefix: ServerIconBox(
                  icon: FLucideIcons.plus,
                  tone: colors.app.success,
                ),
                title: Text(creating.value ? 'Creating key…' : 'Create key'),
                subtitle: Text(
                  keys.length >= _maxKeys
                      ? 'At most $_maxKeys keys; revoke one first'
                      : 'For the extension or an AI agent',
                ),
                suffix: creating.value
                    ? const FCircularProgress.pinwheel()
                    : const Icon(FLucideIcons.chevronRight),
                onPress: busy.value || keys.length >= _maxKeys ? null : create,
              ),
            ],
          ),
          const ServerNote('Long-press a key to show it again on this phone.'),
        ],
        const SectionHeader(title: 'Get the apps'),
        FTileGroup(
          children: [
            FTile(
              prefix: ServerIconBox(
                icon: FLucideIcons.puzzle,
                tone: colors.app.accentTone,
              ),
              title: const Text('Browser extension'),
              subtitle: const Text('Download and install steps'),
              suffix: const Icon(FLucideIcons.chevronRight),
              onPress: () => context.pushNamed(Paths.chromeExtension),
            ),
          ],
        ),
      ],
    );
  }
}

/// The page's shape while the account loads: the offline choice, then a
/// couple of keys.
class _AppsSkeleton extends StatelessWidget {
  const _AppsSkeleton();

  @override
  Widget build(BuildContext context) {
    FTile row(double title, double subtitle) => FTile(
      prefix: const Skeleton(width: 30, height: 30, radius: Radii.sm + 1),
      title: Align(
        alignment: Alignment.centerLeft,
        child: Skeleton(width: title, height: 14),
      ),
      subtitle: Padding(
        padding: const EdgeInsets.only(top: 6),
        child: Align(
          alignment: Alignment.centerLeft,
          child: Skeleton(width: subtitle, height: 11),
        ),
      ),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const SizedBox(height: Space.md),
        FTileGroup(children: [row(170, 120)]),
        const SizedBox(height: Space.lg),
        const Padding(
          padding: EdgeInsets.only(left: 4, bottom: Space.sm),
          child: Skeleton(width: 90, height: 14),
        ),
        FTileGroup(
          divider: FItemDivider.indented,
          children: [row(110, 190), row(130, 160), row(90, 150)],
        ),
      ],
    );
  }
}
