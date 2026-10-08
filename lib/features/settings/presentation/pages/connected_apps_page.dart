import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/utils/toast/common_toast.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/core/widgets/app_dialog.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

/// Access keys for the browser extension and AI agents (MCP), and the
/// controls over what the sign-in service keeps.
class ConnectedAppsPage extends HookConsumerWidget {
  const ConnectedAppsPage({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final account = useState<BridgeAccount?>(null);
    final loading = useState(true);
    final busy = useState(false);
    final loadError = useState<String?>(null);
    final colors = context.theme.colors;

    Future<String> cookieHeader({required bool force}) async {
      final client = await ref
          .read(vClientProvider.notifier)
          .ensureLogin(force: force);
      if (!await fetchIsAuth(client: client)) {
        throw StateError('Sign in to VTOP first.');
      }
      final cookies =
          createPersistedVtopSessionSnapshot(client: client).cookies?.trim() ??
          '';
      if (cookies.isEmpty) throw StateError('Sign in to VTOP first.');
      return cookies;
    }

    Future<T> withService<T>(
      Future<T> Function(VtopBridgeAccountService service, String cookies) run,
    ) async {
      final client = http.Client();
      try {
        final service = VtopBridgeAccountService(client: client);
        return await withFreshVtopSession(
          cookies: cookieHeader,
          run: (cookies) => run(service, cookies),
        );
      } finally {
        client.close();
      }
    }

    Future<void> load() async {
      loading.value = true;
      loadError.value = null;
      try {
        final result = await withService((s, c) => s.account(cookies: c));
        final prefs = await ref.read(settingsProvider.future);
        if (!result.linkedHere) {
          await prefs.setBool(bridgeLinkedSettingKey, false);
        }
        account.value = result;
      } catch (error) {
        loadError.value = '$error';
      } finally {
        loading.value = false;
      }
    }

    useEffect(() {
      unawaited(load());
      return null;
    }, const []);

    Future<Map<String, dynamic>?> consentCredentials() async {
      final user = await ref.read(vtopUserProvider.future);
      if (user is! ConfiguredVtopUser) return null;
      final gmail = await ref
          .read(googleEmailOtpAuthServiceProvider)
          .loadSession();
      return bridgeCredentialsPayload(
        wantCredentials: true,
        serverSignIn: true,
        username: user.username,
        password: user.password,
        gmail: gmail,
        deleteAfterReading: ref.read(emailOtpDeleteAfterReadingProvider),
        sharedClientId: googleOauthClientId,
      );
    }

    /// Links this phone (its FCM token), with the saved credentials when
    /// [withCredentials].
    Future<void> linkPhone({required bool withCredentials}) async {
      final token = await getFcmTokenForCopy();
      if (token == null || token.isEmpty) {
        throw StateError('Notifications are unavailable on this phone.');
      }
      Map<String, dynamic>? credentials;
      if (withCredentials) {
        credentials = await consentCredentials();
        if (credentials == null) {
          throw StateError('Set up Gmail OTP auto-fetch first.');
        }
      }
      await withService(
        (s, c) => s.link(cookies: c, fcmToken: token, credentials: credentials),
      );
      final prefs = await ref.read(settingsProvider.future);
      await prefs.setBool(bridgeLinkedSettingKey, true);
      await prefs.setString(bridgeLastFcmTokenSettingKey, token);
    }

    Future<bool> serverSignInEnabled() async {
      final prefs = await ref.read(settingsProvider.future);
      return prefs.getBool(bridgeServerSignInSettingKey) ?? false;
    }

    Future<void> run(
      String failureTitle,
      Future<void> Function() action,
    ) async {
      if (busy.value) return;
      busy.value = true;
      try {
        await action();
        await load();
      } catch (error) {
        if (context.mounted) dispToast(context, failureTitle, '$error');
      } finally {
        busy.value = false;
      }
    }

    Future<bool> confirm(String title, String body, String action) async {
      final result = await showFDialog<bool>(
        context: context,
        useRootNavigator: true,
        builder: (context, style, animation) => AppDialog(
          animation: animation,
          direction: Axis.horizontal,
          title: Text(title),
          body: Text(body),
          actions: [
            FButton(
              variant: FButtonVariant.outline,
              onPress: () => Navigator.of(context).pop(false),
              child: const Text('Cancel'),
            ),
            FButton(
              variant: FButtonVariant.destructive,
              onPress: () => Navigator.of(context).pop(true),
              child: Text(action),
            ),
          ],
        ),
      );
      return result ?? false;
    }

    /// Linking from an install without the app secret revokes every key, so
    /// ask first when the account is already linked from elsewhere.
    Future<bool> okToRelink() async {
      final current = account.value;
      if (current == null || !current.linked || current.thisPhone) return true;
      return confirm(
        'Link this phone?',
        'The account is linked from another install. Linking here revokes all its access keys and forgets saved credentials.',
        'Link',
      );
    }

    Future<void> createKey() async {
      if (!(account.value?.linkedHere ?? false) && !await okToRelink()) return;
      if (!context.mounted) return;
      final label = await _askLabel(context);
      if (label == null) return;
      await run('Could not create a key', () async {
        if (!(account.value?.linkedHere ?? false)) {
          await linkPhone(withCredentials: await serverSignInEnabled());
        }
        final key = await withService(
          (s, c) => s.createKey(cookies: c, label: label),
        );
        if (context.mounted) await _showNewKey(context, key);
      });
    }

    Future<void> toggleServerSignIn(bool on) async {
      if (on && !await okToRelink()) return;
      await run(on ? 'Could not turn on' : 'Could not turn off', () async {
        final prefs = await ref.read(settingsProvider.future);
        if (on) {
          await linkPhone(withCredentials: true);
        } else {
          await withService((s, c) => s.forgetCredentials(cookies: c));
        }
        await prefs.setBool(bridgeServerSignInSettingKey, on);
      });
    }

    Future<void> saveSettings(BridgeSettings next) => run(
      'Could not save',
      () => withService((s, c) => s.updateSettings(cookies: c, settings: next)),
    );

    Future<void> clearCredentials() async {
      final ok = await confirm(
        'Clear saved credentials?',
        'The server deletes your password and Gmail access and turns offline sign-in off. Your phone signs in instead.',
        'Clear',
      );
      if (ok) await toggleServerSignIn(false);
    }

    Future<void> revoke(BridgeKeyInfo key) async {
      final ok = await confirm(
        'Revoke "${key.label}"?',
        'Apps using this key stop working right away.',
        'Revoke',
      );
      if (!ok) return;
      await run(
        'Could not revoke',
        () => withService((s, c) => s.revokeKey(cookies: c, id: key.id)),
      );
    }

    Future<void> deleteData() async {
      final ok = await confirm(
        'Delete your data?',
        'Removes your keys, cached session and any saved credentials from the sign-in service. Connected apps stop working.',
        'Delete',
      );
      if (!ok) return;
      await run('Could not delete', () async {
        await withService((s, c) => s.deleteAccount(cookies: c));
        final prefs = await ref.read(settingsProvider.future);
        await prefs.setBool(bridgeLinkedSettingKey, false);
        await prefs.setBool(bridgeServerSignInSettingKey, false);
        await prefs.remove(bridgeLastFcmTokenSettingKey);
      });
    }

    final current = account.value;
    final signInOn = useFuture(
      useMemoized(serverSignInEnabled, [current, busy.value]),
    );
    final gmailReady = ref.watch(emailOtpReadyProvider).value ?? false;
    final offlineOn =
        (signInOn.data ?? false) && (current?.savedCredentials ?? false);
    // Settings change only from the linked phone (they need its secret).
    final settings = (current?.linkedHere ?? false) ? current!.settings : null;
    final muted = context.theme.typography.body.sm.copyWith(
      color: colors.mutedForeground,
    );

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(
        Space.md,
        Space.sm,
        Space.md,
        Space.xxl,
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Expanded(
                child: Text(
                  current?.registrationNumber ?? ' ',
                  style: context.theme.typography.body.sm.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              if (current != null)
                _Status(
                  linked: current.phoneLinked && current.thisPhone,
                  tone: current.phoneLinked && current.thisPhone
                      ? colors.app.success
                      : colors.app.warning,
                ),
            ],
          ),
          const SizedBox(height: Space.xs),
          Text(
            'One access key works in the browser extension and in AI agents (MCP). Your phone signs in for them when needed.',
            style: muted,
          ),
          if (loading.value && current == null) ...[
            const SizedBox(height: Space.xl),
            const Center(child: FCircularProgress.pinwheel()),
          ] else if (loadError.value != null && current == null) ...[
            const SizedBox(height: Space.lg),
            FAlert(
              variant: FAlertVariant.destructive,
              title: const Text('Could not reach the sign-in service'),
              subtitle: Text(loadError.value!),
            ),
            const SizedBox(height: Space.sm),
            FButton(
              variant: FButtonVariant.outline,
              onPress: load,
              child: const Text('Try again'),
            ),
          ] else ...[
            const SectionHeader(title: 'Sign-in'),
            FTileGroup(
              children: [
                FTile(
                  prefix: _IconBox(
                    icon: FLucideIcons.keyRound,
                    tone: colors.app.accentTone,
                  ),
                  title: const Text('Offline sign-in'),
                  subtitle: Text(
                    gmailReady
                        ? 'Server can sign in without your phone'
                        : 'Needs Gmail OTP auto-fetch',
                  ),
                  suffix: FSwitch(
                    value: offlineOn,
                    onChange: busy.value || !gmailReady
                        ? null
                        : toggleServerSignIn,
                  ),
                ),
                if (offlineOn && settings != null) ...[
                  FTile(
                    prefix: _IconBox(
                      icon: FLucideIcons.zap,
                      tone: colors.app.accentTone,
                    ),
                    title: const Text('Always use saved credentials'),
                    subtitle: Text(
                      settings.alwaysUseVault
                          ? 'Server signs in right away'
                          : 'Your phone is asked first',
                    ),
                    suffix: FSwitch(
                      value: settings.alwaysUseVault,
                      onChange: busy.value
                          ? null
                          : (on) => saveSettings(
                              settings.copyWith(alwaysUseVault: on),
                            ),
                    ),
                  ),
                  if (!settings.alwaysUseVault)
                    FSelectMenuTile<int>(
                      key: ValueKey('wait-${settings.phoneWaitSecs}'),
                      prefix: _IconBox(
                        icon: FLucideIcons.timer,
                        tone: colors.app.accentTone,
                      ),
                      title: FTappable(child: const Text('Wait for phone')),
                      subtitle: Text(
                        'Phone gets ${settings.phoneWaitSecs} s, then the server signs in',
                      ),
                      enabled: !busy.value,
                      selectControl: FMultiValueControl.managedRadio(
                        initial: settings.phoneWaitSecs,
                        onChange: (value) {
                          if (value.isEmpty ||
                              value.first == settings.phoneWaitSecs) {
                            return;
                          }
                          saveSettings(
                            settings.copyWith(phoneWaitSecs: value.first),
                          );
                        },
                      ),
                      menu: [
                        for (final secs in BridgeSettings.phoneWaitChoices)
                          FSelectTile(title: Text('$secs s'), value: secs),
                      ],
                    ),
                  FSelectMenuTile<int>(
                    key: ValueKey('ttl-${settings.vaultTtlSecs}'),
                    prefix: _IconBox(
                      icon: FLucideIcons.hourglass,
                      tone: colors.app.accentTone,
                    ),
                    title: FTappable(child: const Text('Keep for')),
                    subtitle: Text(_ttlLabel(settings.vaultTtlSecs)),
                    enabled: !busy.value,
                    selectControl: FMultiValueControl.managedRadio(
                      initial: settings.vaultTtlSecs ?? 0,
                      onChange: (value) {
                        if (value.isEmpty) return;
                        final ttl = value.first == 0 ? null : value.first;
                        if (ttl == settings.vaultTtlSecs) return;
                        saveSettings(
                          settings.copyWith(vaultTtlSecs: () => ttl),
                        );
                      },
                    ),
                    menu: [
                      for (final ttl in BridgeSettings.vaultTtlChoices)
                        FSelectTile(
                          title: Text(_ttlChoice(ttl)),
                          value: ttl ?? 0,
                        ),
                    ],
                  ),
                  FTile(
                    prefix: _IconBox(
                      icon: FLucideIcons.eraser,
                      tone: colors.app.warning,
                    ),
                    title: const Text('Clear saved credentials'),
                    subtitle: const Text('Your phone signs in instead'),
                    onPress: busy.value ? null : clearCredentials,
                  ),
                ],
              ],
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(4, Space.sm, 4, 0),
              child: Text(
                'Off: the server never has your password and asks your phone each time. On: the server keeps your password and Gmail access encrypted and signs in itself, reading the OTP from Gmail. Either way, a session is checked with VTOP before an app gets it.',
                style: muted,
              ),
            ),
            SectionHeader(
              title: 'Access keys',
              trailing: Text('${current?.keys.length ?? 0}/10'),
            ),
            FTileGroup(
              divider: FItemDivider.indented,
              children: [
                for (final key in current?.keys ?? const <BridgeKeyInfo>[])
                  FTile(
                    prefix: _IconBox(
                      icon: FLucideIcons.key,
                      tone: colors.app.accentTone,
                    ),
                    title: Text(key.label),
                    subtitle: Text('${key.id} · ${_lastUsed(key.lastUsedAt)}'),
                    suffix: FButton.icon(
                      variant: FButtonVariant.ghost,
                      onPress: busy.value ? null : () => revoke(key),
                      child: const Icon(FLucideIcons.trash2),
                    ),
                  ),
                FTile(
                  prefix: _IconBox(
                    icon: FLucideIcons.plus,
                    tone: colors.app.success,
                  ),
                  title: const Text('Create key'),
                  subtitle: const Text('For the extension or an AI agent'),
                  suffix: busy.value
                      ? const FCircularProgress.pinwheel()
                      : const Icon(FLucideIcons.chevronRight),
                  onPress: busy.value || (current?.keys.length ?? 0) >= 10
                      ? null
                      : createKey,
                ),
              ],
            ),
            const SectionHeader(title: 'Get the apps'),
            FTileGroup(
              children: [
                FTile(
                  prefix: _IconBox(
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
            const SectionHeader(title: 'Your data'),
            FTileGroup(
              divider: FItemDivider.indented,
              children: [
                FTile(
                  prefix: _IconBox(
                    icon: FLucideIcons.smartphone,
                    tone: colors.app.warning,
                  ),
                  title: const Text('Reconnect this phone'),
                  subtitle: Text(switch (current) {
                    BridgeAccount(linkedHere: true) =>
                      'Updates this phone\'s notification token',
                    BridgeAccount(linked: true) =>
                      'Linked from another install; this revokes its keys',
                    _ => 'Your phone is not linked yet',
                  }),
                  onPress: busy.value
                      ? null
                      : () async {
                          if (!await okToRelink()) return;
                          await run(
                            'Could not reconnect',
                            () async => linkPhone(
                              withCredentials:
                                  await serverSignInEnabled() && gmailReady,
                            ),
                          );
                        },
                ),
                FTile(
                  prefix: _IconBox(
                    icon: FLucideIcons.trash2,
                    tone: colors.app.danger,
                  ),
                  title: Text(
                    'Delete my data',
                    style: TextStyle(color: colors.app.danger.base),
                  ),
                  subtitle: const Text('Keys, cached session, credentials'),
                  onPress: busy.value ? null : deleteData,
                ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

String _lastUsed(DateTime? at) {
  if (at == null) return 'never used';
  final ago = DateTime.now().toUtc().difference(at);
  if (ago.inMinutes < 1) return 'used just now';
  if (ago.inHours < 1) return 'used ${ago.inMinutes}m ago';
  if (ago.inDays < 1) return 'used ${ago.inHours}h ago';
  return 'used ${ago.inDays}d ago';
}

Future<String?> _askLabel(BuildContext context) {
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

Future<void> _showNewKey(BuildContext context, BridgeKey key) {
  return showFDialog<void>(
    context: context,
    useRootNavigator: true,
    builder: (context, style, animation) => AppDialog(
      animation: animation,
      title: const Text('Copy your key now'),
      body: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text(
            "It won't be shown again. Paste the key into the browser extension, or the MCP URL into your AI agent.",
          ),
          const SizedBox(height: Space.md),
          _CopyRow(label: 'Access key', value: key.key),
          _CopyRow(label: 'MCP URL (contains the key)', value: key.agentUrl),
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

class _Status extends StatelessWidget {
  const _Status({required this.linked, required this.tone});

  final bool linked;
  final Tone tone;

  @override
  Widget build(BuildContext context) => Row(
    mainAxisSize: MainAxisSize.min,
    children: [
      Container(
        width: 7,
        height: 7,
        decoration: BoxDecoration(color: tone.base, shape: BoxShape.circle),
      ),
      const SizedBox(width: Space.xs + 2),
      Text(
        linked ? 'phone linked' : 'phone not linked',
        style: context.theme.typography.body.xs.copyWith(color: tone.onSubtle),
      ),
    ],
  );
}

class _IconBox extends StatelessWidget {
  const _IconBox({required this.icon, required this.tone});

  final IconData icon;
  final Tone tone;

  @override
  Widget build(BuildContext context) => Container(
    width: 30,
    height: 30,
    decoration: BoxDecoration(
      color: tone.subtle,
      borderRadius: BorderRadius.circular(Radii.sm + 1),
    ),
    child: Icon(icon, size: 16, color: tone.onSubtle),
  );
}

String _ttlChoice(int? secs) => switch (secs) {
  null => 'Until it stops working',
  86400 => '1 day',
  172800 => '2 days',
  _ => '${secs ~/ 3600} h',
};

String _ttlLabel(int? secs) => secs == null
    ? 'Until VTOP or Google rejects it'
    : '${_ttlChoice(secs)} after your phone last sent it';
