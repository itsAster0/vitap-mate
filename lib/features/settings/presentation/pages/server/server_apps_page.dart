import 'package:flutter/material.dart';
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
      await run('Could not create a key', () async {
        final key = await actions.createKey(label);
        if (context.mounted) await showAccessKey(context, key, fresh: true);
      });
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
      await run('Could not revoke', () => actions.revokeKey(key.id));
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

    Future<void> chooseOffline(_Offline mode) => run(
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

    String offlineSubtitle(_Offline mode) => switch (mode) {
      _Offline.phone => 'Only this phone',
      _Offline.wait10 => 'Wait 10s, then the server',
      _Offline.wait20 => 'Wait 20s, then the server',
      _Offline.server => 'Server signs in right away',
    };

    return ServerSubPage(
      title: 'Connected apps',
      children: [
        const ServerNote(
          'One access key works in the browser extension and in AI agents (MCP). The phone that created a key signs in for it when needed.',
        ),
        if (account.isLoading)
          const Padding(
            padding: EdgeInsets.symmetric(vertical: Space.xxl),
            child: Center(child: FCircularProgress.pinwheel()),
          )
        else if (account.hasError)
          FAlert(
            variant: FAlertVariant.destructive,
            title: const Text('Could not load your account'),
            subtitle: Text('${account.error}'),
          )
        else ...[
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
          FTileGroup(
            children: [
              FSelectMenuTile<_Offline>(
                prefix: ServerIconBox(
                  icon: FLucideIcons.smartphone,
                  tone: colors.app.accentTone,
                ),
                title: FTappable(
                  child: const Text('When the phone is offline'),
                ),
                subtitle: Text(offlineSubtitle(offline)),
                selectControl: FMultiValueControl.managedRadio(
                  initial: offline,
                  onChange: busy.value
                      ? null
                      : (value) {
                          if (value.isNotEmpty) chooseOffline(value.first);
                        },
                ),
                menu: const [
                  FSelectTile(
                    title: Text('Only this phone'),
                    subtitle: Text(
                      'Apps wait for this phone to sign in. Nothing is sealed on the server.',
                      softWrap: true,
                      maxLines: 3,
                    ),
                    value: _Offline.phone,
                  ),
                  FSelectTile(
                    title: Text('Wait 10s, then the server'),
                    subtitle: Text(
                      'Give this phone 10 seconds, then the server signs in.',
                      softWrap: true,
                      maxLines: 3,
                    ),
                    value: _Offline.wait10,
                  ),
                  FSelectTile(
                    title: Text('Wait 20s, then the server'),
                    subtitle: Text(
                      'Give this phone 20 seconds, then the server signs in.',
                      softWrap: true,
                      maxLines: 3,
                    ),
                    value: _Offline.wait20,
                  ),
                  FSelectTile(
                    title: Text('Server signs in right away'),
                    subtitle: Text(
                      'Fastest for apps; this phone is not asked first.',
                      softWrap: true,
                      maxLines: 3,
                    ),
                    value: _Offline.server,
                  ),
                ],
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
                    [
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
                  suffix: FButton.icon(
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
                title: const Text('Create key'),
                subtitle: const Text('For the extension or an AI agent'),
                suffix: busy.value
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
