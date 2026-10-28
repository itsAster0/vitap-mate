/// "Fetch on the server" ([VtopServerMode.server]): the engine signs in
/// with the phone's password and fetches pages on the session it hands
/// back, sending only the parsed page. The server keeps nothing: the
/// session lives in the phone's client, which still sends what changes VTOP
/// and downloads files itself. If the server cannot be reached, the phone
/// does the work instead; VTOP's own refusals are never retried that way.
library;

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:http/http.dart' as http;
import 'package:vitapmate/observability/log.dart';
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

/// One page over the experimental UDP transport (`rust/src/api/wire.rs`).
typedef WirePageFetch =
    Future<WirePage> Function({
      required String sessionJson,
      required String name,
      String? semester,
      required Map<String, String> args,
      String? dictionary,
    });

/// [WirePageFetch] through Rust, to the build's gateway (`vtop_udp.dart`).
Future<WirePage> rustWirePage({
  required String sessionJson,
  required String name,
  String? semester,
  required Map<String, String> args,
  String? dictionary,
}) async => wireFetchPage(
  config: await udpWireConfig(),
  sessionJson: sessionJson,
  name: name,
  semester: semester,
  args: [for (final e in args.entries) (e.key, e.value)],
  dictionary: dictionary,
  timeoutMs: ServerVtopApi.wireTimeout.inMilliseconds,
);

Future<bool> _never() async => false;

/// The edge's `/v1/vtop/*` routes; page fetches over the UDP gateway
/// instead when [useWire] says so (see `vtop_udp.dart`).
class ServerVtopApi {
  ServerVtopApi({
    required http.Client client,
    String? baseUrl,
    PageDictionary? pages,
    WirePageFetch? wire,
    Future<bool> Function() useWire = _never,
  }) : _client = client,
       _baseUrl = baseUrl,
       _pages = pages,
       _wire = wire,
       _useWire = useWire;

  final http.Client _client;

  /// Pages come as JSON without one.
  final PageDictionary? _pages;

  /// Fixed for tests; otherwise the server URL in use at each call.
  final String? _baseUrl;

  /// Over the edge's own 32 s, which is over the engine's VTOP limits.
  static const timeout = Duration(seconds: 35);

  /// A page over UDP at 1 kbit/s can take this long.
  static const wireTimeout = Duration(seconds: 45);

  /// After the gateway does not answer, pages use HTTPS for this long.
  static const wireRetryAfter = Duration(minutes: 10);

  final WirePageFetch? _wire;
  final Future<bool> Function() _useWire;
  DateTime? _wireOffUntil;

  Future<Map<String, dynamic>> fetch({
    required SessionState session,
    required String name,
    String? semester,
    Map<String, String> args = const {},
  }) async {
    final pages = _pages;
    final response =
        await _sendWire(session, name, semester, args, pages?.id()) ??
        await _send('/v1/vtop/fetch', {
          'session': sessionToServerJson(session),
          'name': name,
          'semester': ?semester,
          'args': args,
          'dictionary': ?pages?.id(),
        });
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

  Future<ServerLogin> login(String username, String password) async => _login(
    await _post('/v1/vtop/login', {'username': username, 'password': password}),
  );

  Future<ServerLogin> submitOtp(SessionState pending, String otp) async =>
      _login(
        await _post('/v1/vtop/login/otp', {
          'session': sessionToServerJson(pending),
          'otp': otp,
        }),
      );

  Future<ServerLogin> resendOtp(SessionState pending) async => _login(
    await _post('/v1/vtop/login/resend', {
      'session': sessionToServerJson(pending),
    }),
  );

  ServerLogin _login(Map<String, dynamic> body) {
    final session = body['session'];
    if (session is! Map<String, dynamic>) {
      throw const ServerVtopUnavailable('the server sent no session');
    }
    final state = sessionFromServerJson(session);
    if (body['status'] == 'otp_required') {
      final at = body['issuedAt'];
      return ServerOtpRequired(
        state,
        '${body['message'] ?? 'OTP required'}',
        at is num ? BigInt.from(at.toInt()) : BigInt.zero,
      );
    }
    return ServerSignedIn(state);
  }

  Future<Map<String, dynamic>> _post(
    String path,
    Map<String, Object?> body,
  ) async => _json(await _send(path, body));

  /// The page over UDP, shaped as the edge's answer; null to use HTTPS:
  /// the mode is off, the gateway did not answer lately, or the page does
  /// not suit UDP. A page that started arriving but did not finish is a
  /// timeout, as over HTTPS (another try over HTTPS would only be slower).
  Future<http.Response?> _sendWire(
    SessionState session,
    String name,
    String? semester,
    Map<String, String> args,
    String? dictionary,
  ) async {
    final wire = _wire;
    if (wire == null || !await _useWire()) return null;
    final off = _wireOffUntil;
    if (off != null && DateTime.now().isBefore(off)) return null;
    final WirePage page;
    try {
      page = await wire(
        sessionJson: jsonEncode(sessionToServerJson(session)),
        name: name,
        semester: semester,
        args: args,
        dictionary: dictionary,
      );
    } on WireFetchError catch (error) {
      switch (error) {
        case WireFetchError_Timeout():
          throw const VtopError.vtopUnavailable(
            'The server took too long to answer.',
          );
        case WireFetchError_Unreachable():
          _wireOffUntil = DateTime.now().add(wireRetryAfter);
          _log.warn('$name: the UDP gateway did not answer; using HTTPS');
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
    // Too large for UDP: the gateway said so without sending the page.
    if (page.status == 413) return null;
    return http.Response.bytes(
      page.body,
      page.status,
      headers: {
        'content-type': page.packed ? packedPageType : 'application/json',
      },
    );
  }

  Future<http.Response> _send(String path, Map<String, Object?> body) async {
    final base = (_baseUrl ?? await currentServerUrl()).replaceFirst(
      RegExp(r'/+$'),
      '',
    );
    if (base.isEmpty) throw const ServerVtopUnavailable('no server URL');
    final http.Response response;
    try {
      response = await _client
          .post(
            Uri.parse('$base$path'),
            headers: {'content-type': 'application/json'},
            body: jsonEncode(body),
          )
          .timeout(timeout);
    } on TimeoutException {
      throw const VtopError.vtopUnavailable(
        'The server took too long to answer.',
      );
    } catch (error) {
      throw ServerVtopUnavailable('$error');
    }
    return response;
  }

  /// The JSON of a 2xx [response]; a refusal is thrown as [serverError].
  Map<String, dynamic> _json(http.Response response) {
    Object? decoded;
    try {
      decoded = jsonDecode(response.body);
    } catch (_) {}
    final json = decoded is Map<String, dynamic>
        ? decoded
        : const <String, dynamic>{};
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
