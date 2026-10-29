/// "Fetch on the server" ([VtopServerMode.server]): the engine signs in
/// with the phone's password and fetches pages on the session it hands
/// back, sending only the parsed page. The server keeps nothing: the
/// session lives in the phone's client, which still sends what changes VTOP
/// and downloads files itself. If the server cannot be reached, the phone
/// does the work instead; VTOP's own refusals are never retried that way.
library;

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:http/http.dart' as http;
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/vtop/edge/server_status.dart' show EdgeIdentity;
import 'package:vitapmate/vtop/route/server_url.dart';
import 'package:vitapmate/vtop/route/vtop_proxy.dart';
import 'package:vitapmate/vtop/route/vtop_udp.dart';
import 'package:vitapmate/vtop/backend/server_json.dart';
import 'package:vitapmate/vtop/backend/vtop_authenticator.dart';
import 'package:vitapmate/vtop/backend/vtop_backend.dart';
import 'package:vitapmate/vtop/backend/vtop_query.dart';
import 'package:vitapmate/src/api/server_page.dart' as server_page;
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart' as vtop_api;
import 'package:vitapmate/src/api/wire.dart';

const _log = SourceLogger('vtop.server');

/// The pages the engine fetches (`rust/vtop-bridge/src/pages.rs`). Others,
/// like `vtop_session`, are the phone's own.
const serverPages = {
  'semesters',
  'attendance',
  'full_attendance',
  'timetable',
  'marks',
  'exam_schedule',
  'grades',
  'academic_calendar',
  'grade_details',
  'grade_history',
  'biometric',
  'profile',
  'mentor',
  'general_outing',
  'weekend_outing',
  'outing_pass',
  'course_page_courses',
  'course_page_classes',
  'course_page_detail',
  'course_file_info',
  'course_plan_info',
};

/// The server could not do it (unreachable, not deployed, or failing):
/// the phone tries itself.
class ServerVtopUnavailable implements Exception {
  const ServerVtopUnavailable(this.reason);

  final String reason;

  @override
  String toString() => 'server VTOP unavailable: $reason';
}

/// How a sign-in step ended on the server.
sealed class ServerLogin {
  const ServerLogin(this.session);

  final SessionState session;
}

class ServerSignedIn extends ServerLogin {
  const ServerSignedIn(super.session);
}

/// VTOP wants the emailed OTP; [session] is the half-done sign-in.
class ServerOtpRequired extends ServerLogin {
  const ServerOtpRequired(super.session, this.message, this.issuedAt);

  final String message;
  final BigInt issuedAt;
}

/// The content type of a page the engine packed with [PageDictionary].
const packedPageType = 'application/vnd.vmate.page+br';

/// The page dictionary (`rust/vtop-shared/src/page_dictionary.rs`): the
/// engine packs a page with it, about 30% smaller than plain Brotli, when
/// the phone names the same [id].
class PageDictionary {
  const PageDictionary({required this.id, required this.unpack});

  /// Which dictionary the phone holds.
  final String Function() id;

  /// A packed page as its JSON text.
  final String Function(String name, Uint8List packed) unpack;
}

/// The app's own dictionary, in Rust.
final rustPageDictionary = PageDictionary(
  id: server_page.serverPageDictionary,
  unpack: (name, packed) =>
      server_page.serverPageUnpack(name: name, packed: packed),
);

/// One request over the experimental UDP transport (`rust/src/api/wire.rs`):
/// a page on `sessionJson`'s session, or a sign-in (empty `sessionJson`). A
/// [secret] one goes only on a forward-secret session.
typedef WirePageFetch =
    Future<WirePage> Function({
      required String sessionJson,
      required String name,
      String? semester,
      required Map<String, String> args,
      String? dictionary,
      String? account,
      bool secret,
      Duration? timeout,
    });

/// [WirePageFetch] through Rust, to the gateway the server names
/// (`vtop_udp.dart`). When the gateway has another key, the key is asked
/// for again and the page tried once more with a new one.
Future<WirePage> rustWirePage({
  required String sessionJson,
  required String name,
  String? semester,
  required Map<String, String> args,
  String? dictionary,
  String? account,
  bool secret = false,
  Duration? timeout,
}) async {
  Future<WirePage> send(UdpGateway gateway) async => wireFetchPage(
    config: await udpWireConfig(gateway),
    sessionJson: sessionJson,
    name: name,
    semester: semester,
    args: [for (final e in args.entries) (e.key, e.value)],
    dictionary: dictionary,
    accountJson: account,
    secret: secret,
    timeoutMs: (timeout ?? Duration(seconds: await currentTunable(udpTimeout)))
        .inMilliseconds,
    flow: currentFlow,
  );
  final gateway = await currentUdpGateway();
  if (gateway == null) {
    throw const WireFetchError.unsupported('no UDP gateway from the server');
  }
  try {
    return await send(gateway);
  } on WireFetchError_WrongKey {
    final fresh = await currentUdpGateway(refresh: true);
    if (fresh == null || fresh.publicKey == gateway.publicKey) rethrow;
    Log.info('vtop.udp', 'the gateway has a new key; trying it');
    return send(fresh);
  }
}

Future<bool> _never() async => false;

Future<EdgeIdentity?> _unlinked() async => null;

Future<void> _ignore(SessionState _) async {}

/// The header a renewed session comes back in, from [ServerVtopApi._sendWire]
/// to [ServerVtopApi.fetch].
const _renewedSessionHeader = 'x-renewed-session';

/// The edge's `/v1/vtop/*` routes; page fetches and sign-ins over the UDP
/// gateway instead when [useWire] says so (see `vtop_udp.dart`).
class ServerVtopApi {
  ServerVtopApi({
    required http.Client client,
    String? baseUrl,
    PageDictionary? pages,
    WirePageFetch? wire,
    Future<bool> Function() useWire = _never,
    Future<EdgeIdentity?> Function() identity = _unlinked,
    Future<bool> Function() useServerSignIn = _never,
    Future<void> Function(SessionState session) onSession = _ignore,
  }) : _client = client,
       _baseUrl = baseUrl,
       _pages = pages,
       _wire = wire,
       _useWire = useWire,
       _identity = identity,
       _useServerSignIn = useServerSignIn,
       _onSession = onSession;

  final http.Client _client;

  /// Pages come as JSON without one.
  final PageDictionary? _pages;

  /// Fixed for tests; otherwise the server URL in use at each call.
  final String? _baseUrl;

  /// After the gateway does not answer, pages use HTTPS for this long.
  static const wireRetryAfter = Duration(minutes: 5);

  /// Until when UDP is skipped, after the gateway did not answer. One
  /// gateway, so shared; shown and cleared in Developer → Debug.
  static final wireOffUntil = ValueNotifier<DateTime?>(null);

  final WirePageFetch? _wire;
  final Future<bool> Function() _useWire;

  /// This phone's link to its account, for a sign-in by the server.
  final Future<EdgeIdentity?> Function() _identity;

  /// Whether the server signs in for this phone with the password it keeps
  /// (`bridgeServerSignInSettingKey`).
  final Future<bool> Function() _useServerSignIn;

  /// Takes a session the gateway renewed in a fetch.
  final Future<void> Function(SessionState session) _onSession;

  /// `/v1/app/login`'s body when the server signs in for this phone, else
  /// null.
  Future<String?> _account() async {
    if (!await _useServerSignIn()) return null;
    final identity = await _identity();
    if (identity == null) return null;
    return jsonEncode({
      'registrationNumber': identity.registrationNumber,
      'appSecret': identity.appSecret,
    });
  }

  /// Over the edge's wait for a sign-in by the server, which waits for the
  /// OTP mail.
  static const serverLoginTimeout = Duration(seconds: 110);

  Future<Map<String, dynamic>> fetch({
    required SessionState session,
    required String name,
    String? semester,
    Map<String, String> args = const {},
  }) async {
    final pages = _pages;
    final response =
        await _sendWire(
          sessionJson: jsonEncode(sessionToServerJson(session)),
          name: name,
          semester: semester,
          args: args,
          dictionary: pages?.id(),
          account: await _account(),
        ) ??
        await _send('/v1/vtop/fetch', {
          'session': sessionToServerJson(session),
          'name': name,
          'semester': ?semester,
          'args': args,
          'dictionary': ?pages?.id(),
        });
    if (response.headers[_renewedSessionHeader] case final renewed?) {
      await _onSession(sessionFromServerJson(jsonObject(renewed)));
    }
    final Object? page;
    if (pages != null && response.headers['content-type'] == packedPageType) {
      try {
        page = jsonDecode(pages.unpack(name, response.bodyBytes));
      } catch (error) {
        throw ServerVtopUnavailable('the packed page does not unpack: $error');
      }
    } else {
      page = _json(response)['data'];
    }
    final data = rustJsonToDart(page);
    if (data is! Map<String, dynamic>) {
      throw const ServerVtopUnavailable('the server sent no page');
    }
    return data;
  }

  Future<ServerLogin> login(String username, String password) =>
      _login('/v1/vtop/login', {'username': username, 'password': password});

  Future<ServerLogin> submitOtp(SessionState pending, String otp) => _login(
    '/v1/vtop/login/otp',
    {'session': sessionToServerJson(pending), 'otp': otp},
  );

  Future<ServerLogin> resendOtp(SessionState pending) => _login(
    '/v1/vtop/login/resend',
    {'session': sessionToServerJson(pending)},
  );

  /// A sign-in by the server with the password it keeps, the OTP read
  /// from forwarded mail or Gmail, so the phone sends no password; over UDP
  /// in UDP mode, else HTTPS. Null to sign in the usual way: server
  /// sign-in is off, the phone is not linked, or the server cannot sign in.
  /// A rejected password or a timeout is thrown as it is: no other try.
  Future<SessionState?> loginOnServer() async {
    final account = await _account();
    if (account == null) return null;
    try {
      final response =
          await _sendWire(
            name: '/v1/app/login',
            args: {'body': account},
            secret: true,
            timeout: serverLoginTimeout,
          ) ??
          await _send(
            '/v1/app/login',
            jsonObject(account),
            timeout: serverLoginTimeout,
          );
      return switch (_json(response)['session']) {
        // Without its CSRF token (an older server) the session would not
        // count as signed in, and the phone would only ask again.
        {'csrf_token': String _} && final Map<String, dynamic> session =>
          sessionFromServerJson(session),
        _ => throw const ServerVtopUnavailable('no CSRF token in the session'),
      };
    } on ServerVtopUnavailable catch (error) {
      _log.info('no sign-in by the server ($error); signing in as usual');
      return null;
    }
  }

  /// A sign-in step over UDP (on a forward-secret session) when the mode
  /// is on, else HTTPS.
  Future<ServerLogin> _login(String path, Map<String, Object?> request) async {
    final body = _json(
      await _sendWire(
            name: path,
            args: {'body': jsonEncode(request)},
            secret: true,
          ) ??
          await _send(path, request),
    );
    final state = switch (body['session']) {
      final Map<String, dynamic> session => sessionFromServerJson(session),
      _ => throw const ServerVtopUnavailable('the server sent no session'),
    };
    if (body['status'] != 'otp_required') return ServerSignedIn(state);
    return ServerOtpRequired(
      state,
      '${body['message'] ?? 'OTP required'}',
      switch (body['issuedAt']) {
        final num at => BigInt.from(at.toInt()),
        _ => BigInt.zero,
      },
    );
  }

  /// The request over UDP, shaped as the edge's answer; null to use
  /// HTTPS: the mode is off, the gateway did not answer lately, or the
  /// request does not suit UDP. One that started but did not finish is a
  /// timeout, as over HTTPS: another try over HTTPS would only be slower,
  /// and a sign-in tried again would mail a second OTP.
  Future<http.Response?> _sendWire({
    String sessionJson = '',
    required String name,
    String? semester,
    Map<String, String> args = const {},
    String? dictionary,
    String? account,
    bool secret = false,
    Duration? timeout,
  }) async {
    final wire = _wire;
    if (wire == null || !await _useWire()) return null;
    if (wireOffUntil.value case final off? when DateTime.now().isBefore(off)) {
      return null;
    }
    final WirePage page;
    try {
      page = await wire(
        sessionJson: sessionJson,
        name: name,
        semester: semester,
        args: args,
        dictionary: dictionary,
        account: account,
        secret: secret,
        timeout: timeout,
      );
    } on WireFetchError catch (error) {
      switch (error) {
        case WireFetchError_Timeout():
          throw const VtopError.vtopUnavailable(
            'The server took too long to answer.',
          );
        case WireFetchError_Unreachable():
          wireOffUntil.value = DateTime.now().add(wireRetryAfter);
          _log.warn('$name: the UDP gateway did not answer; using HTTPS');
        case WireFetchError_WrongKey():
          wireOffUntil.value = DateTime.now().add(wireRetryAfter);
          _log.warn('$name: the UDP gateway has another key; using HTTPS');
        case WireFetchError_Unsupported(:final field0) ||
            WireFetchError_Failed(:final field0):
          _log.warn('$name over UDP: $field0; using HTTPS');
      }
      return null;
    } catch (error) {
      _log.warn('$name over UDP: $error; using HTTPS');
      return null;
    }
    _log.info(
      '$name over UDP: ${page.status} in ${page.elapsedMs} ms, '
      '${page.wireBytes} B${page.handshake ? ', new session' : ''}',
    );
    // Too large for UDP, or the gateway is busy: it said so without
    // fetching the page.
    if (page.status case 413 || 429) return null;
    // An older gateway cannot read this request: HTTPS for a while.
    if (page.status == 426 ||
        page.status == 400 && utf8.decode(page.body).contains('bad_request')) {
      wireOffUntil.value = DateTime.now().add(wireRetryAfter);
      _log.warn('$name: the UDP gateway cannot read the request; using HTTPS');
      return null;
    }
    return http.Response.bytes(
      page.body,
      page.status,
      headers: {
        'content-type': page.packed ? packedPageType : 'application/json',
        _renewedSessionHeader: ?page.session,
      },
    );
  }

  Future<http.Response> _send(
    String path,
    Map<String, Object?> body, {
    Duration? timeout,
  }) async {
    final base = (_baseUrl ?? await currentServerUrl()).replaceFirst(
      RegExp(r'/+$'),
      '',
    );
    if (base.isEmpty) throw const ServerVtopUnavailable('no server URL');
    try {
      return await _client
          .post(
            Uri.parse('$base$path'),
            headers: {'content-type': 'application/json'},
            body: jsonEncode(body),
          )
          .timeout(
            timeout ?? Duration(seconds: await currentTunable(serverTimeout)),
          );
    } on TimeoutException {
      throw const VtopError.vtopUnavailable(
        'The server took too long to answer.',
      );
    } catch (error) {
      throw ServerVtopUnavailable('$error');
    }
  }

  /// The JSON of a 2xx [response]; a refusal is thrown as [serverError].
  Map<String, dynamic> _json(http.Response response) {
    final json = jsonObject(response.body);
    final status = response.statusCode;
    if (status >= 200 && status < 300) return json;
    throw serverError(
      status,
      '${json['code'] ?? ''}',
      '${json['error'] ?? ''}',
    );
  }
}

/// A refusal from the server as the app's [VtopError], so the usual
/// handling (signing in again, the OTP prompt, "VTOP is down") follows;
/// anything that is the server's own trouble is [ServerVtopUnavailable].
Exception serverError(int status, String code, String message) =>
    switch (code) {
      'session_expired' => const VtopError.sessionExpired(),
      'invalid_credentials' => const VtopError.invalidCredentials(),
      'authentication_failed' => VtopError.authenticationFailed(message),
      'vtop_unavailable' ||
      'vtop_timeout' => VtopError.vtopUnavailable(message),
      'parse_error' => VtopError.parseError(message),
      'bad_arguments' => VtopError.configurationError(message),
      'vtop_error' => VtopError.vtopServerError(message),
      _ => ServerVtopUnavailable('HTTP $status ${code.isEmpty ? '' : code}'),
    };

/// The signed-in session of [client], for [ServerVtopBackend].
Future<SessionState?> signedInSession(VtopClient client) async =>
    await vtop_api.fetchIsAuth(client: client)
    ? vtop_api.exportSessionState(client: client)
    : null;

/// Reads through the server in server mode; everything else, and every
/// read when the server cannot be reached, on [_local].
class ServerVtopBackend implements VtopBackend {
  ServerVtopBackend(
    this._local, {
    required Future<SessionState?> Function() session,
    required ServerVtopApi api,
    Future<VtopServerMode> Function() mode = currentVtopMode,
  }) : _session = session,
       _api = api,
       _mode = mode;

  final VtopBackend _local;

  /// The phone's signed-in session; null when it is not signed in.
  final Future<SessionState?> Function() _session;
  final ServerVtopApi _api;
  final Future<VtopServerMode> Function() _mode;

  @override
  String get name => 'phone or server';

  @override
  Future<T> fetch<T>(VtopQuery<T> query) async {
    if (!serverPages.contains(query.name) ||
        await _mode() != VtopServerMode.server) {
      return _local.fetch(query);
    }
    // Not signed in yet: the caller signs in (through the server) and
    // asks again.
    final session = await _session();
    if (session == null) throw const VtopError.sessionExpired();
    try {
      final data = await _api.fetch(
        session: session,
        name: query.name,
        semester: query.semester,
        args: query.args,
      );
      return query.fromJson(data);
    } on ServerVtopUnavailable catch (error) {
      _log.warn('${query.name}: $error; fetching on the phone');
      return _local.fetch(query);
    }
  }

  /// Changes to VTOP are always sent by the phone.
  @override
  Future<T> run<T>(VtopAction<T> action) => _local.run(action);
}

/// Signs in on the server in server mode (the password is sent, never
/// kept), leaving the session in the phone's client; on the phone
/// ([_local]) otherwise, or when the server cannot be reached.
class ServerVtopAuthenticator implements VtopAuthenticator {
  ServerVtopAuthenticator(
    this._local, {
    required Future<({String username, String password})?> Function()
    credentials,
    required ServerVtopApi api,
    Future<VtopServerMode> Function() mode = currentVtopMode,
  }) : _credentials = credentials,
       _api = api,
       _mode = mode;

  final VtopAuthenticator _local;
  final Future<({String username, String password})?> Function() _credentials;
  final ServerVtopApi _api;
  final Future<VtopServerMode> Function() _mode;

  @override
  Future<void> login(VtopClient client) async {
    final credentials = await _credentials();
    if (credentials == null || await _mode() != VtopServerMode.server) {
      return _local.login(client);
    }
    _pendingOnServer = false;
    // UDP mode: the server signs in with the password it keeps.
    if (await _api.loginOnServer() case final session?) {
      return _apply(client, ServerSignedIn(session));
    }
    final ServerLogin outcome;
    try {
      outcome = await _api.login(credentials.username, credentials.password);
    } on ServerVtopUnavailable catch (error) {
      _log.warn('sign-in: $error; signing in on the phone');
      return _local.login(client);
    }
    _apply(client, outcome);
  }

  @override
  Future<void> submitOtp(VtopClient client, String otp) async {
    final pending = vtop_api.exportSessionState(client: client);
    // A sign-in the phone started (the server was down) ends on the phone.
    if (!_pendingOnServer) return _local.submitOtp(client, otp);
    _apply(client, await _api.submitOtp(pending, otp));
  }

  @override
  Future<void> resendOtp(VtopClient client) async {
    final pending = vtop_api.exportSessionState(client: client);
    if (!_pendingOnServer) return _local.resendOtp(client);
    final outcome = await _api.resendOtp(pending);
    vtop_api.vtopClientResumeSession(client: client, session: outcome.session);
  }

  /// The OTP being asked for is the server's sign-in's, not one the phone
  /// started itself (when the server was down).
  bool _pendingOnServer = false;

  /// Puts the server's session in [client]; an OTP request is thrown as
  /// the phone's own sign-in would, so the usual prompt asks for it.
  void _apply(VtopClient client, ServerLogin outcome) {
    vtop_api.vtopClientResumeSession(client: client, session: outcome.session);
    switch (outcome) {
      case ServerSignedIn():
        _pendingOnServer = false;
        _log.info('signed in on the server');
      case ServerOtpRequired(:final message, :final issuedAt):
        _pendingOnServer = true;
        _log.info('the server sign-in needs the OTP');
        throw VtopError.otpRequired(message, issuedAt);
    }
  }
}
