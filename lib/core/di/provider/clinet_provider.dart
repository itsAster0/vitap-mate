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
  @override
  Future<VtopClient> build() async {
    String? username = await ref.watch(
      vtopUserProvider.selectAsync((user) => user.username),
    );
    String? password = await ref.watch(
      vtopUserProvider.selectAsync((user) => user.password),
    );
    final uname = username!;

    final storedSession = await loadStoredVtopSession(uname);
    PersistedVtopSession? persistedSession;
    if (storedSession != null) {
      AppLogger.instance.info(
        'client.session',
        'found saved session for $uname (age=${storedSession.age.inMinutes}m, ttl=${vtopSessionReuseTtl.inMinutes}m)',
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
          'restoring saved session for $uname with ${storedSession.snapshot.cookies.length} cookies',
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

  Future<void> ensureLogin({bool force = false}) async {
    VtopClient client = await future;
    VtopUserEntity user = await ref.watch(vtopUserProvider.future);
    if (!force) {
      if (await fetchIsAuth(client: client)) return;
    }
    if (!user.isValid) throw VtopError.invalidCredentials();
    final featureFlags = await ref.read(featureFlagsControllerProvider.future);
    if (!await featureFlags.isEnabled("try-login")) {
      throw FeatureDisabledException("Login is Disabled");
    }
    AppLogger.instance.info('client.auth', 'login attempt started');

    try {
      await ref.read(globalAsyncQueueProvider.notifier).run(
        "vtop_login_${user.username}",
        () async {
          try {
            await vtopClientLogin(client: client);
          } catch (e) {
            if (!isSecurityOtpRequiredError(e)) rethrow;
            await ref
                .read(vtopOtpChallengeProvider.notifier)
                .requestOtp(
                  client: client,
                  message:
                      'Additional verification required. OTP sent to your registered email.',
                );
          }
        },
      );
      final snapshot = createPersistedVtopSessionSnapshot(
        client: client,
        ttl: vtopSessionReuseTtl,
      );
      await saveStoredVtopSession(snapshot);
      AppLogger.instance.info(
        'client.session',
        'saved refreshed session for ${snapshot.username} with ${snapshot.cookies.length} cookies and ${snapshot.headers.length} headers',
      );
      AppLogger.instance.info('client.auth', 'login attempt finished');
    } catch (e) {
      final uname = user.username;
      if (e == VtopError.invalidCredentials()) {
        AppLogger.instance.warning(
          'client.auth',
          'credentials rejected; marking stored user as invalid',
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
          'cleared saved session after a terminal authentication failure',
        );
      }

      AppLogger.instance.error('client.auth', 'login failed: $e');

      rethrow;
    }
  }
}
