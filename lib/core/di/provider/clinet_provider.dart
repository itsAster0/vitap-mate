import 'dart:async';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/logging/app_logger.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';
import 'package:vitapmate/core/utils/vtop_login_with_otp.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/core/utils/users/vtop_users_utils.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
part 'clinet_provider.g.dart';

@Riverpod(keepAlive: true)
class VClient extends _$VClient {
  int _loginFlowCounter = 0;

  @override
  Future<VtopClient> build() async {
    String? username = await ref.watch(
      vtopUserProvider.selectAsync((user) => user.username),
    );
    String? password = await ref.watch(
      vtopUserProvider.selectAsync((user) => user.password),
    );
    final uname = username!.toUpperCase();
    final StoredVtopSession? storedSession = await loadStoredVtopSession(uname);
    PersistedVtopSession? persistedSession;
    if (storedSession != null) {
      AppLogger.instance.info(
        'client.session',
        'found saved session for $uname (age=${storedSession.age.inMinutes}m, ttl=${storedSession.ttl.inMinutes}m)',
      );
      if (storedSession.isExpired) {
        AppLogger.instance.info(
          'client.session',
          'saved session expired for $uname; clearing it before login',
        );
        await clearStoredVtopSession(uname);
      } else {
        persistedSession = storedSession.snapshot;
        AppLogger.instance.info(
          'client.session',
          'restoring saved session for $uname with ${storedSession.snapshot.cookies?.length ?? 0} cookies',
        );
      }
    }

    return getVtopClient(
      username: uname,
      password: password!,
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
      AppLogger.instance.warning(
        'client.session',
        '${flowLabel ?? 'session.persist'} skipped because cookie header was empty',
      );
      return;
    }
    await saveStoredVtopSession(snapshot);
    AppLogger.instance.info(
      'client.session',
      '${flowLabel ?? 'session.persist'} saved snapshot for ${snapshot.username} (cookieHeaderLength=${snapshot.cookies?.length ?? 0})',
    );
  }

  Future<void> ensureLogin({
    bool force = false,
    bool promptForOtp = true,
  }) async {
    VtopClient client = await future;
    VtopUserEntity user = await ref.watch(vtopUserProvider.future);
    final flowId = ++_loginFlowCounter;
    final flowLabel = 'auth.flow#$flowId';
    if (!force) {
      if (await fetchIsAuth(client: client)) {
        AppLogger.instance.info(
          'client.auth',
          '$flowLabel reused an authenticated in-memory session',
        );
        await _persistCurrentSession(
          client,
          flowLabel: '$flowLabel session.persist',
        );
        return;
      }
    }
    if (!user.isValid) {
      AppLogger.instance.warning(
        'client.auth',
        '$flowLabel blocked because stored credentials are marked invalid',
      );
      throw const VtopError.authenticationFailed(
        'Your saved VTOP credentials need attention. Check them in Settings.',
      );
    }
    final featureFlags = await ref.read(featureFlagsControllerProvider.future);
    if (!await featureFlags.isEnabled("try-login")) {
      AppLogger.instance.warning(
        'client.auth',
        '$flowLabel blocked because try-login feature flag is disabled',
      );
      throw FeatureDisabledException("Login is Disabled");
    }
    AppLogger.instance.info(
      'client.auth',
      '$flowLabel started (force=$force, promptForOtp=$promptForOtp, user=${user.username?.toUpperCase() ?? '<unknown>'})',
    );

    try {
      await ref.read(globalAsyncQueueProvider.notifier).run(
        "vtop_login_${user.username}",
        () async {
          AppLogger.instance.info(
            'client.auth',
            '$flowLabel entered serialized login queue',
          );
          try {
            await vtopClientLogin(client: client);
          } catch (e) {
            if (!isSecurityOtpRequiredError(e)) rethrow;
            AppLogger.instance.info(
              'client.auth',
              '$flowLabel requires OTP verification',
            );
            if (!promptForOtp &&
                !await ref
                    .read(vtopOtpChallengeProvider.notifier)
                    .canAutoFetchFromEmail()) {
              AppLogger.instance.warning(
                'client.auth',
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
            AppLogger.instance.info(
              'client.auth',
              '$flowLabel OTP verification completed',
            );
          }
        },
      );
      await _persistCurrentSession(
        client,
        flowLabel: '$flowLabel session.persist',
      );
      AppLogger.instance.info(
        'client.auth',
        '$flowLabel finished successfully',
      );
    } catch (e) {
      final uname = user.username;
      if (e == VtopError.invalidCredentials()) {
        AppLogger.instance.warning(
          'client.auth',
          '$flowLabel credentials rejected; marking stored user as invalid',
        );
        await ref
            .read(vtopusersutilsProvider.notifier)
            .vtopUserSave(user.copyWith(isValid: false));
        ref.invalidate(vtopUserProvider);
      }
      if (uname != null &&
          uname.isNotEmpty &&
          e is VtopError &&
          e is! VtopError_NetworkError) {
        await clearStoredVtopSession(uname);
        AppLogger.instance.warning(
          'client.session',
          '$flowLabel cleared saved session after a terminal authentication failure',
        );
      }

      AppLogger.instance.error('client.auth', '$flowLabel failed: $e');

      rethrow;
    }
  }
}
