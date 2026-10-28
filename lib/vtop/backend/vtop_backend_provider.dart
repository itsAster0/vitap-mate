import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/vtop/users/vtop_users_utils.dart';
import 'package:vitapmate/vtop/users/saved_credentials.dart';
import 'package:vitapmate/vtop/clinet_provider.dart';
import 'package:vitapmate/vtop/vtop_user_provider.dart';
import 'package:vitapmate/vtop/backend/vtop_authenticator.dart';
import 'package:vitapmate/vtop/backend/local_vtop_backend.dart';
import 'package:vitapmate/vtop/edge/server_status.dart';
import 'package:vitapmate/vtop/backend/relogin_vtop_backend.dart';
import 'package:vitapmate/vtop/route/vtop_udp.dart';
import 'package:vitapmate/vtop/backend/server_vtop.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:vitapmate/vtop/entity/vtop_user_entity.dart';
import 'package:vitapmate/vtop/backend/vtop_backend.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';

part 'vtop_backend_provider.g.dart';

/// Whether a re-login may show the OTP prompt. Headless containers
/// (background sync, the FCM cookie bridge) override this to false.
@Riverpod(keepAlive: true)
bool vtopLoginPromptAllowed(Ref ref) => true;

/// Who this phone is to the server: its app secret and the student's
/// registration number. Null when the phone is not linked.
Future<EdgeIdentity?> edgeIdentity(Ref ref) async {
  final secret = await const SecureBridgeAppSecretStore().read();
  if (secret == null || secret.isEmpty) return null;
  var number = await _knownRegistrationNumber(ref);
  if (number == null) {
    // Not learnt yet: sign in once (the login saves it). Never the
    // username, which can differ from the registration number.
    try {
      await ref
          .read(vClientProvider.notifier)
          .ensureLogin(promptForOtp: ref.read(vtopLoginPromptAllowedProvider));
    } catch (_) {
      return null;
    }
    number = await _knownRegistrationNumber(ref);
  }
  if (number == null) return null;
  return (appSecret: secret, registrationNumber: number.toUpperCase());
}

/// The registration number already known, from the saved account or a live
/// session, trimmed; null when neither has it yet.
Future<String?> _knownRegistrationNumber(Ref ref) async {
  final saved = (await ref.read(
    vtopUserProvider.future,
  )).registrationNumber?.trim();
  if (saved != null && saved.isNotEmpty) return saved;
  final session = (await _deviceSession(ref))?.registrationNumber?.trim();
  return (session != null && session.isNotEmpty) ? session : null;
}

Future<SessionState?> _deviceSession(Ref ref) async {
  final client = await ref.read(vClientProvider.future);
  if (!await vtop_api.fetchIsAuth(client: client)) return null;
  return vtop_api.exportSessionState(client: client);
}

/// VTOP through the phone's own client (directly, or through the server's
/// compressing proxy), or read by the server in "Fetch on the server" mode;
/// see `vtop_proxy.dart`. An expired session triggers one re-login and
/// retry.
@Riverpod(keepAlive: true)
VtopBackend vtopBackend(Ref ref) {
  Future<VtopClient> client() => ref.read(vClientProvider.future);
  return ReloginVtopBackend(
    ServerVtopBackend(
      LocalVtopBackend(client),
      session: () async => signedInSession(await client()),
      api: ServerVtopApi(
        client: sharedAppHttpClient,
        pages: rustPageDictionary,
        wire: rustWirePage,
        useWire: currentVtopUdp,
      ),
    ),
    relogin: () => ref
        .read(vClientProvider.notifier)
        .ensureLogin(
          force: true,
          promptForOtp: ref.read(vtopLoginPromptAllowedProvider),
        ),
    // VTOP turned the saved password down: mark it, so the app asks again.
    onCredentialsRejected: () async {
      final users = ref.read(vtopusersutilsProvider.notifier);
      if (await rejectSavedCredentials(
        load: users.vtopUserDefault,
        save: users.vtopUserSave,
      )) {
        ref.invalidate(vtopUserProvider);
      }
    },
  );
}

/// Logs in on the device, or on the server in "Fetch on the server" mode.
@Riverpod(keepAlive: true)
VtopAuthenticator vtopAuthenticator(Ref ref) => ServerVtopAuthenticator(
  const LocalVtopAuthenticator(),
  credentials: () async {
    final user = await ref.read(vtopUserProvider.future);
    if (user is! StoredVtopUser) return null;
    return (username: user.username.toUpperCase(), password: user.password);
  },
  api: ServerVtopApi(client: sharedAppHttpClient),
);
