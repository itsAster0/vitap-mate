import 'package:http/http.dart' as http;
import 'package:vitapmate/core/vtop_backend/vtop_server_client.dart';
import 'package:vitapmate/core/vtop_backend/vtop_server_settings.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;

/// Signs a [VtopClient] in, on the device or through a vtop-server.
///
/// Both leave the signed-in session in the local client, so persisting,
/// the WebView cookie bridge and local fetches keep working unchanged. Both
/// throw [VtopError.otpRequired] when VTOP wants the emailed code, which the
/// existing OTP prompt then sends to [submitOtp].
abstract interface class VtopAuthenticator {
  Future<void> login(VtopClient client);
  Future<void> submitOtp(VtopClient client, String otp);
  Future<void> resendOtp(VtopClient client);
}

/// Logs in with the bundled Rust library; the password stays on the phone.
class LocalVtopAuthenticator implements VtopAuthenticator {
  const LocalVtopAuthenticator();

  @override
  Future<void> login(VtopClient client) =>
      vtop_api.vtopClientLogin(client: client);

  @override
  Future<void> submitOtp(VtopClient client, String otp) =>
      vtop_api.vtopClientSubmitSecurityOtp(client: client, otpCode: otp);

  @override
  Future<void> resendOtp(VtopClient client) =>
      vtop_api.vtopClientResendSecurityOtp(client: client);
}

/// Stored credentials for a server login.
typedef VtopCredentials = ({String username, String password});

/// Gmail access lent to vtop-server for one login so it can answer VTOP's
/// emailed OTP itself.
typedef GmailLoginAccess = ({
  String accessToken,
  int expiresAtUnix,
  bool deleteAfterReading,
});

/// Logs in through vtop-server's stateless `/v1/auth/*`. The server returns
/// the whole session (cookies, CSRF token, registration number, login time)
/// and keeps nothing; the session is loaded into the local client.
class RemoteVtopAuthenticator implements VtopAuthenticator {
  RemoteVtopAuthenticator({
    required VtopServerSettings settings,
    required Future<VtopCredentials> Function() credentials,
    Future<GmailLoginAccess?> Function()? gmail,
    Future<void> Function()? onGmailTokenRejected,
    http.Client? httpClient,
    void Function(VtopClient, SessionState)? resume,
    SessionState Function(VtopClient)? export,
  }) : _server = VtopServerClient(
         settings,
         httpClient: httpClient,
         timeout: const Duration(seconds: 150),
       ),
       _credentials = credentials,
       _gmail = gmail,
       _onGmailTokenRejected = onGmailTokenRejected,
       _resume =
           resume ??
           ((client, session) => vtop_api.vtopClientResumeSession(
             client: client,
             session: session,
           )),
       _export =
           export ?? ((client) => vtop_api.exportSessionState(client: client));

  final VtopServerClient _server;
  final Future<VtopCredentials> Function() _credentials;

  /// A fresh Gmail token when email OTP is set up, else null.
  final Future<GmailLoginAccess?> Function()? _gmail;

  /// Called when the server says the Gmail token was rejected, so it is
  /// refreshed before the phone's own OTP auto-fetch runs.
  final Future<void> Function()? _onGmailTokenRejected;

  /// Loads a session into the local client (FRB by default; swappable so
  /// tests run without the Rust library).
  final void Function(VtopClient, SessionState) _resume;
  final SessionState Function(VtopClient) _export;

  void _adopt(VtopClient client, Map<String, dynamic> reply) {
    final session = reply['session'];
    if (session is! Map<String, dynamic>) {
      throw const VtopError.invalidResponse();
    }
    _resume(client, sessionFromServerJson(session));
  }

  @override
  Future<void> login(VtopClient client) async {
    final credentials = await _credentials();
    GmailLoginAccess? gmail;
    try {
      gmail = await _gmail?.call();
    } catch (_) {
      // No Gmail help this time; the OTP prompt still works.
      gmail = null;
    }
    final (reply, _) = await _server.post('auth/login', {
      'username': credentials.username.toUpperCase(),
      'password': credentials.password,
      if (gmail != null)
        'gmail': {
          'access_token': gmail.accessToken,
          'expires_at': gmail.expiresAtUnix,
          'delete_after_reading': gmail.deleteAfterReading,
        },
    });
    _adopt(client, reply);
    if (reply['status'] == 'otp_required') {
      if (reply['gmail'] == 'gmail_unauthorized') {
        try {
          await _onGmailTokenRejected?.call();
        } catch (_) {}
      }
      final issuedAt = reply['issued_at'];
      throw VtopError.otpRequired(
        '${reply['message'] ?? 'Additional verification is required.'}',
        BigInt.from(issuedAt is num ? issuedAt.toInt() : 0),
      );
    }
  }

  @override
  Future<void> submitOtp(VtopClient client, String otp) async {
    final session = _export(client);
    final (reply, _) = await _server.post('auth/otp', {
      'session': sessionToServerJson(session),
      'otp': otp,
    });
    _adopt(client, reply);
  }

  @override
  Future<void> resendOtp(VtopClient client) async {
    final session = _export(client);
    final (reply, _) = await _server.post('auth/otp/resend', {
      'session': sessionToServerJson(session),
    });
    _adopt(client, reply);
  }
}
