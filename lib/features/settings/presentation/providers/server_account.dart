import 'dart:async';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/edge/edge_device.dart';
import 'package:vitapmate/core/utils/edge/edge_events.dart';
import 'package:vitapmate/core/utils/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';

/// The account calls the Server pages make. They need a VTOP session from
/// the phone (the server checks whose account it is).
abstract interface class ServerAccountApi {
  Future<BridgeAccount> account();

  /// Links this phone (its FCM token), with the saved credentials when
  /// [withCredentials].
  Future<void> link({required bool withCredentials});
  Future<BridgeKey> createKey(String label);
  Future<void> revokeKey(String id);
  Future<void> removePhone(BridgePhone phone);
  Future<void> forgetCredentials();
  Future<void> updateSettings(BridgeSettings settings);
  Future<void> deleteAccount();
}

final serverAccountApiProvider = Provider<ServerAccountApi>(
  _BridgeServerAccountApi.new,
);

class _BridgeServerAccountApi implements ServerAccountApi {
  _BridgeServerAccountApi(this._ref);

  final Ref _ref;

  Future<String> _cookieHeader({required bool force}) async {
    final client = await _ref
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

  Future<T> _withService<T>(
    Future<T> Function(VtopBridgeAccountService service, String cookies) run,
  ) async {
    final client = http.Client();
    try {
      final service = VtopBridgeAccountService(client: client);
      return await withFreshVtopSession(
        cookies: _cookieHeader,
        run: (cookies) => run(service, cookies),
      );
    } finally {
      client.close();
    }
  }

  @override
  Future<BridgeAccount> account() =>
      _withService((s, c) => s.account(cookies: c));

  Future<Map<String, dynamic>?> _credentials() async {
    final user = await _ref.read(vtopUserProvider.future);
    if (user is! ConfiguredVtopUser) return null;
    final forwarding = await _ref.read(mailForwardingServiceProvider).isReady();
    return bridgeCredentialsPayload(
      wantCredentials: true,
      serverSignIn: true,
      username: user.username,
      password: user.password,
      forwarding: forwarding,
    );
  }

  @override
  Future<void> link({required bool withCredentials}) async {
    final token = await getFcmTokenForCopy();
    if (token == null || token.isEmpty) {
      throw StateError('Notifications are unavailable on this phone.');
    }
    Map<String, dynamic>? credentials;
    if (withCredentials) {
      credentials = await _credentials();
      if (credentials == null) {
        throw StateError('Set up OTP email forwarding first.');
      }
    }
    final semid = (await _ref.read(vtopUserProvider.future)).semid;
    final label = await currentDeviceLabel();
    await _withService((s, c) async {
      // The server keeps credentials only for the owner of a forwarding
      // address, so tie it to this student first.
      if (credentials != null) {
        await _ref.read(mailForwardingServiceProvider).claimOwner(cookies: c);
      }
      // Registers this install again if the server lost it.
      await EdgeDeviceService().withInstall(
        token,
        (device) => s.link(
          cookies: c,
          fcmToken: token,
          credentials: credentials,
          semesterId: semid,
          device: device,
          label: label,
        ),
      );
    });
    unawaited(EdgeEventChannel.instance.connect());
    final prefs = await _ref.read(settingsProvider.future);
    await prefs.setString(bridgeLastFcmTokenSettingKey, token);
    if (semid != null) {
      await prefs.setString(bridgeLastSemesterSettingKey, semid);
    }
  }

  @override
  Future<BridgeKey> createKey(String label) =>
      _withService((s, c) => s.createKey(cookies: c, label: label));

  @override
  Future<void> revokeKey(String id) =>
      _withService((s, c) => s.revokeKey(cookies: c, id: id));

  @override
  Future<void> removePhone(BridgePhone phone) =>
      _withService((s, c) => s.removePhone(cookies: c, phone: phone));

  @override
  Future<void> forgetCredentials() =>
      _withService((s, c) => s.forgetCredentials(cookies: c));

  @override
  Future<void> updateSettings(BridgeSettings settings) =>
      _withService((s, c) => s.updateSettings(cookies: c, settings: settings));

  @override
  Future<void> deleteAccount() =>
      _withService((s, c) => s.deleteAccount(cookies: c));
}

/// The account on the server, and what the Server pages change on it.
class ServerAccount extends AsyncNotifier<BridgeAccount> {
  ServerAccountApi get _api => ref.read(serverAccountApiProvider);

  Future<void> _setPref(String key, bool value) async {
    final prefs = await ref.read(settingsProvider.future);
    await prefs.setBool(key, value);
    ref.invalidate(settingsProvider);
  }

  @override
  Future<BridgeAccount> build() async {
    final account = await ref.read(serverAccountApiProvider).account();
    final prefs = await ref.read(settingsProvider.future);
    if (!account.linkedHere) {
      if (prefs.getBool(bridgeLinkedSettingKey) ?? false) {
        await prefs.setBool(bridgeLinkedSettingKey, false);
      }
    } else if (account.semesterId == null) {
      // Linked before semesters were shared: send it now.
      await prefs.remove(bridgeLastSemesterSettingKey);
      unawaited(
        syncBridgeSemester((await ref.read(vtopUserProvider.future)).semid),
      );
    }
    return account;
  }

  /// Runs [action] with the provider kept alive: a page may only read the
  /// notifier (the mode page does), and this provider disposes itself once
  /// nothing watches it, mid-request.
  Future<T> _kept<T>(Future<T> Function() action) async {
    final link = ref.keepAlive();
    try {
      return await action();
    } finally {
      link.close();
    }
  }

  Future<void> _reload() async {
    state = AsyncData(await _api.account());
  }

  Future<void> link({required bool withCredentials}) => _kept(() async {
    await _api.link(withCredentials: withCredentials);
    await _setPref(bridgeLinkedSettingKey, true);
    await _reload();
  });

  Future<BridgeKey> createKey(String label) => _kept(() async {
    if (!(state.value?.linkedHere ?? false)) {
      final prefs = await ref.read(settingsProvider.future);
      await link(
        withCredentials: prefs.getBool(bridgeServerSignInSettingKey) ?? false,
      );
    }
    final key = await _api.createKey(label);
    await _reload();
    return key;
  });

  Future<void> revokeKey(String id) => _kept(() async {
    await _api.revokeKey(id);
    await _reload();
  });

  Future<void> removePhone(BridgePhone phone) => _kept(() async {
    await _api.removePhone(phone);
    if (phone.thisPhone) await _setPref(bridgeLinkedSettingKey, false);
    await _reload();
  });

  /// On: links with the saved password, then checks the server kept it.
  /// Off: the server forgets it.
  Future<void> setServerSignIn(bool on) => _kept(() async {
    if (on) {
      await link(withCredentials: true);
      if (!(state.value?.savedCredentials ?? false)) {
        throw StateError(
          'The server could not keep your password. Check OTP email forwarding and try again.',
        );
      }
    } else {
      await _api.forgetCredentials();
      await _reload();
    }
    await _setPref(bridgeServerSignInSettingKey, on);
  });

  Future<void> updateSettings(BridgeSettings settings) => _kept(() async {
    await _api.updateSettings(settings);
    await _reload();
  });

  Future<void> deleteData() => _kept(() async {
    await _api.deleteAccount();
    final prefs = await ref.read(settingsProvider.future);
    await prefs.setBool(bridgeLinkedSettingKey, false);
    await prefs.setBool(bridgeServerSignInSettingKey, false);
    await prefs.remove(bridgeLastFcmTokenSettingKey);
    ref.invalidate(settingsProvider);
    await _reload();
  });
}

final serverAccountProvider =
    AsyncNotifierProvider.autoDispose<ServerAccount, BridgeAccount>(
      ServerAccount.new,
    );
