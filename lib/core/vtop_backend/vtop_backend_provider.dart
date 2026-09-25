import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_otp_challenge_provider.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:vitapmate/core/utils/vtop_session_store.dart';
import 'package:vitapmate/core/vtop_backend/vtop_authenticator.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/vtop_backend/local_vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/relogin_vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/remote_vtop_backend.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

part 'vtop_backend_provider.g.dart';

/// Whether a re-login may show the OTP prompt. Headless containers
/// (background sync, the FCM cookie bridge) override this to false.
@Riverpod(keepAlive: true)
bool vtopLoginPromptAllowed(Ref ref) => true;

/// The server when one is configured in Settings, otherwise the bundled
/// Rust library. Either way an expired session triggers one re-login and
/// retry. Rebuilt when the setting changes.
@Riverpod(keepAlive: true)
VtopBackend vtopBackend(Ref ref) {
  final server = ref.watch(vtopServerSettingsProvider);
  final VtopBackend inner = server.isEnabled
      ? RemoteVtopBackend(
          settings: server,
          session: () async {
            final client = await ref.read(vClientProvider.future);
            if (!await vtop_api.fetchIsAuth(client: client)) return null;
            return vtop_api.exportSessionState(client: client);
          },
          onSessionChanged: (session) async {
            // VTOP rotated cookies during a server request: keep the phone's
            // client and saved session in step with what VTOP now expects.
            final client = await ref.read(vClientProvider.future);
            vtop_api.vtopClientResumeSession(client: client, session: session);
            await saveStoredVtopSession(
              createPersistedVtopSessionSnapshot(client: client),
            );
          },
        )
      : LocalVtopBackend(() => ref.read(vClientProvider.future));

  return ReloginVtopBackend(
    inner,
    relogin: () => ref
        .read(vClientProvider.notifier)
        .ensureLogin(
          force: true,
          promptForOtp: ref.read(vtopLoginPromptAllowedProvider),
        ),
  );
}

/// Logs in on the device, or through the server when one is configured.
@Riverpod(keepAlive: true)
VtopAuthenticator vtopAuthenticator(Ref ref) {
  final server = ref.watch(vtopServerSettingsProvider);
  if (!server.isEnabled) return const LocalVtopAuthenticator();
  return RemoteVtopAuthenticator(
    settings: server,
    gmail: () async {
      final canUseGmail = await ref
          .read(vtopOtpChallengeProvider.notifier)
          .canAutoFetchFromEmail();
      if (!canUseGmail) return null;
      final access = await ref
          .read(googleEmailOtpAuthServiceProvider)
          .accessForServerLogin();
      if (access == null) return null;
      return (
        accessToken: access.accessToken,
        expiresAtUnix: access.expiresAtUnix,
        deleteAfterReading: ref.read(emailOtpDeleteAfterReadingProvider),
      );
    },
    onGmailTokenRejected: () async {
      await ref
          .read(googleEmailOtpAuthServiceProvider)
          .refreshIfNeeded(force: true);
    },
    credentials: () async {
      final user = await ref.read(vtopUserProvider.future);
      if (user is! StoredVtopUser) {
        throw const VtopError.configurationError(
          'A configured VTOP account is required.',
        );
      }
      return (username: user.username, password: user.password);
    },
  );
}
