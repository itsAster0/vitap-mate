import 'dart:async';

import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/vtop/global_async_queue_provider.dart';
import 'package:vitapmate/vtop/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/vtop/exceptions.dart';
import 'package:vitapmate/observability/app_logger.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/vtop/entity/vtop_user_entity.dart';
import 'package:vitapmate/app/feature_flags.dart';
import 'package:vitapmate/vtop/vtop_login_with_otp.dart';
import 'package:vitapmate/vtop/vtop_session_store.dart';
import 'package:vitapmate/vtop/users/vtop_users_utils.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:vitapmate/vtop/http/vtop_platform_http.dart';
part 'clinet_provider.g.dart';

const _auth = SourceLogger('client.auth');
const _session = SourceLogger('client.session');

@Riverpod(keepAlive: true)
class VClient extends _$VClient {
  int _loginFlowCounter = 0;

  @override
  Future<VtopClient> build() async {
    final user = await ref.watch(vtopUserProvider.future);
    if (user is! StoredVtopUser) {
      throw StateError('A configured VTOP account is required.');
    }
    final uname = user.username.toUpperCase();
    final StoredVtopSession? storedSession = await loadStoredVtopSession(uname);
    PersistedVtopSession? persistedSession;
    if (storedSession != null) {
      _session.info(
        'found saved session for $uname (age=${storedSession.age.inMinutes}m, ttl=${storedSession.ttl.inMinutes}m)',
      );
      if (storedSession.isExpired) {
        _session.info(
          'saved session expired for $uname; clearing it before login',
        );
        await clearStoredVtopSession(uname);
      } else {
        persistedSession = storedSession.snapshot;
        _session.info(
          'restoring saved session for $uname with ${storedSession.snapshot.cookies?.length ?? 0} cookies',
        );
      }
    }

    return createVtopClient(
      username: uname,
      password: user.password,
      persistedSession: persistedSession,
    );
  }

  void replaceVClinet(VtopClient vclinet) {
    state = AsyncData(vclinet);
  }

  Future<void> _persistCurrentSession(
    VtopClient client, {
    String? flowLabel,
  }) async {
    final snapshot = createPersistedVtopSessionSnapshot(client: client);
    if (snapshot.cookies?.trim().isEmpty ?? true) {
      _session.warning(
        '${flowLabel ?? 'session.persist'} skipped because cookie header was empty',
      );
      return;
    }
    await saveStoredVtopSession(snapshot);
    await _rememberRegistrationNumber(snapshot.registrationNumber);
    _session.info(
      '${flowLabel ?? 'session.persist'} saved snapshot for ${snapshot.username} (cookieHeaderLength=${snapshot.cookies?.length ?? 0})',
    );
  }

  /// Saves the registration number VTOP just gave, so the app need not sign
  /// in again only to learn it. It can differ from the login username.
  Future<void> _rememberRegistrationNumber(String? number) async {
    final regno = number?.trim();
    if (regno == null || regno.isEmpty) return;
    final users = ref.read(vtopusersutilsProvider.notifier);
    final user = await users.vtopUserDefault();
    if (user == null || user.registrationNumber == regno) return;
    await users.vtopUserSave(user.withRegistrationNumber(regno));
    ref.invalidate(vtopUserProvider);
  }

  /// True when [client] is signed in and its last real login is younger
  /// than the Session Reuse setting. The login time comes from Rust, which
  /// sets it only when a password (+ OTP) login completes; sessions restored
  /// without a known login time are reused until VTOP rejects them.
  Future<bool> _canReuseSession(VtopClient client, String flowLabel) async {
    if (!await fetchIsAuth(client: client)) return false;
    final loggedInAt = exportSessionState(client: client).loggedInAt;
    if (loggedInAt == null) return true;
    final age = DateTime.now().toUtc().difference(
      DateTime.fromMillisecondsSinceEpoch(
        loggedInAt.toInt() * 1000,
        isUtc: true,
      ),
    );
    final ttl = ref.read(vtopSessionReuseTtlProvider);
    if (age <= ttl) return true;
    _auth.info(
      '$flowLabel session is ${age.inMinutes}m old (limit ${ttl.inMinutes}m); logging in again',
    );
    return false;
  }

  Future<VtopClient> ensureLogin({
    bool force = false,
    bool promptForOtp = true,
  }) async {
    VtopClient client = await future;
    VtopUserEntity user = await ref.watch(vtopUserProvider.future);
    final flowId = ++_loginFlowCounter;
    final flowLabel = 'auth.flow#$flowId';
    if (!force) {
      if (await _canReuseSession(client, flowLabel)) {
        _auth.info('$flowLabel reused an authenticated in-memory session');
        await _persistCurrentSession(
          client,
          flowLabel: '$flowLabel session.persist',
        );
        return client;
      }
    }
    if (!user.isValid) {
      _auth.warning(
        '$flowLabel blocked because stored credentials are marked invalid',
      );
      throw const VtopError.authenticationFailed(
        'Your saved VTOP credentials need attention. Check them in Settings.',
      );
    }
    final featureFlags = await ref.read(featureFlagsControllerProvider.future);
    if (!await featureFlags.isEnabled("try-login")) {
      _auth.warning(
        '$flowLabel blocked because try-login feature flag is disabled',
      );
      throw FeatureDisabledException("Login is Disabled");
    }
    _auth.info(
      '$flowLabel started (force=$force, promptForOtp=$promptForOtp, user=${user.username?.toUpperCase() ?? '<unknown>'})',
    );

    try {
      await ref.read(globalAsyncQueueProvider.notifier).run(
        "vtop_login_${user.username}",
        () async {
          _auth.info('$flowLabel entered serialized login queue');
          if (!force && await _canReuseSession(client, flowLabel)) {
            _auth.info(
              '$flowLabel skipped full login because the session became authenticated while waiting for the queue',
            );
            return;
          }
          try {
            await ref.read(vtopAuthenticatorProvider).login(client);
          } catch (e) {
            if (!isSecurityOtpRequiredError(e)) rethrow;
            _auth.info('$flowLabel requires OTP verification');
            if (!promptForOtp &&
                !await ref
                    .read(vtopOtpChallengeProvider.notifier)
                    .canAutoFetchFromEmail()) {
              _auth.warning(
                '$flowLabel cannot continue because OTP prompt is disabled and email autofetch is unavailable',
              );
              rethrow;
            }
            await ref
                .read(vtopOtpChallengeProvider.notifier)
                .requestOtp(
                  client: client,
                  message:
                      'Additional verification required. OTP sent to your registered email.',
                  otpRequiredAt: securityOtpRequiredAt(e),
                  logContext: flowLabel,
                );
            _auth.info('$flowLabel OTP verification completed');
          }
        },
      );
      await _persistCurrentSession(
        client,
        flowLabel: '$flowLabel session.persist',
      );
      _auth.info('$flowLabel finished successfully');
      return client;
    } catch (e) {
      final uname = user.username;
      if (e == VtopError.invalidCredentials()) {
        _auth.warning(
          '$flowLabel credentials rejected; marking stored user as invalid',
        );
        await ref
            .read(vtopusersutilsProvider.notifier)
            .vtopUserSave(user.rejectCredentials());
        ref.invalidate(vtopUserProvider);
        // The server must not keep trying the same password either.
        unawaited(_reportCredentialsRejected(user.username));
      }
      if (uname != null &&
          uname.isNotEmpty &&
          e is VtopError &&
          e is! VtopError_NetworkError) {
        await clearStoredVtopSession(uname);
        _session.warning(
          '$flowLabel cleared saved session after a terminal authentication failure',
        );
      }

      _auth.error('$flowLabel failed: $e');

      rethrow;
    }
  }
}

Future<void> _reportCredentialsRejected(String? username) async {
  if (username == null || username.trim().isEmpty) return;
  try {
    await VtopBridgeAccountService(
      client: sharedAppHttpClient,
    ).reportCredentialsRejected(registrationNumber: username);
  } catch (error) {
    _auth.warning(
      'could not tell the server about rejected credentials: $error',
    );
  }
}
