import 'dart:async';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/vtop/client_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/vtop/edge/edge_device.dart';
import 'package:vitapmate/vtop/edge/edge_events.dart';
import 'package:vitapmate/vtop/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/vtop/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/vtop/entity/vtop_user_entity.dart';
import 'package:vitapmate/platform/fcm_cookie_bridge_service.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/vtop/vtop_session_store.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';

/// The account calls the Server pages make. They need a VTOP session from
/// the phone (the server checks whose account it is).
abstract interface class ServerAccountApi {
  Future<BridgeAccount> account();

  /// Links this phone (its FCM token), with the saved credentials when
  /// [withCredentials].
  Future<void> link({required bool withCredentials});
  Future<BridgeKey> createKey(String label, {bool outings = false});
  Future<void> setKeyOutings(String id, bool outings);
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
    return await signedInCookies(client) ??
        (throw StateError('Sign in to VTOP first.'));
  }

  Future<T> _withService<T>(
    Future<T> Function(VtopBridgeAccountService service, String cookies) run,
  ) async {
    final service = VtopBridgeAccountService(
      client: _ref.read(appHttpClientProvider),
    );
    return withFreshVtopSession(
      cookies: _cookieHeader,
      run: (cookies) => run(service, cookies),
    );
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
      gmail: forwarding
          ? null
          : await serverGmailGrant(
              _ref.read(googleEmailOtpAuthServiceProvider),
              await _ref.read(settingsProvider.future),
            ),
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
        throw StateError('Set up OTP email forwarding or connect Gmail first.');
      }
    }
    final semid = (await _ref.read(vtopUserProvider.future)).semid;
    final label = await currentDeviceLabel();
    await _withService((s, c) async {
      // Without Gmail access the server keeps credentials only for the
      // owner of a forwarding address, so tie it to this student first.
      if (credentials != null && credentials['gmail'] == null) {
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
  Future<BridgeKey> createKey(String label, {bool outings = false}) =>
      _withService(
        (s, c) => s.createKey(cookies: c, label: label, outings: outings),
      );

  @override
  Future<void> setKeyOutings(String id, bool outings) => _withService(
    (s, c) => s.setKeyOutings(cookies: c, id: id, outings: outings),
  );

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

  /// Runs one server [call], then reloads the account.
  Future<void> _apply(Future<void> Function() call) => _kept(() async {
    await call();
    await _reload();
  });

  Future<void> link({required bool withCredentials}) => _kept(() async {
    await _api.link(withCredentials: withCredentials);
    await _setPref(bridgeLinkedSettingKey, true);
    await _reload();
  });

  Future<BridgeKey> createKey(String label, {bool outings = false}) => _kept(
    () async {
      if (!(state.value?.linkedHere ?? false)) {
        final prefs = await ref.read(settingsProvider.future);
        await link(
          withCredentials: prefs.getBool(bridgeServerSignInSettingKey) ?? false,
        );
      }
      final key = await _api.createKey(label, outings: outings);
      await _reload();
      return key;
    },
  );

  Future<void> setKeyOutings(String id, bool outings) =>
      _apply(() => _api.setKeyOutings(id, outings));

  Future<void> revokeKey(String id) => _apply(() => _api.revokeKey(id));

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
          'The server could not keep your password. Check OTP email forwarding or Gmail and try again.',
        );
      }
    } else {
      await _api.forgetCredentials();
      await _reload();
    }
    await _setPref(bridgeServerSignInSettingKey, on);
  });

  Future<void> updateSettings(BridgeSettings settings) =>
      _apply(() => _api.updateSettings(settings));

  /// Saves [choice]: the password is sealed first when the server is to
  /// sign in (needs OTP forwarding or Gmail), and forgotten for
  /// [SignInChoice.phone].
  Future<void> setSignIn(SignInChoice choice) async {
    if (choice == .phone) return setServerSignIn(false);
    if (!(state.value?.savedCredentials ?? false)) await setServerSignIn(true);
    await updateSettings(
      BridgeSettings(
        alwaysUseVault: choice == .server,
        phoneWaitSecs: choice == .wait10 ? 10 : 20,
      ),
    );
  }

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

/// Who signs in to VTOP when connected apps need a session, and whether
/// the server signs in for this phone when it fetches on the server: one
/// choice, kept on the server with the sealed password.
enum SignInChoice { phone, wait10, wait20, server }

/// [account]'s choice: [SignInChoice.phone] when it keeps no password.
SignInChoice signInChoice(BridgeAccount? account) {
  if (!(account?.savedCredentials ?? false)) return .phone;
  final settings = account!.settings;
  if (settings.alwaysUseVault) return .server;
  return settings.phoneWaitSecs <= 10 ? .wait10 : .wait20;
}

final serverAccountProvider =
    AsyncNotifierProvider.autoDispose<ServerAccount, BridgeAccount>(
      ServerAccount.new,
    );
