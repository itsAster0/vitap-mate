import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:http/http.dart' as http;
import 'package:vitapmate/core/vtop_backend/vtop_server_settings.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// Talks JSON to a vtop-server and turns its failures into [VtopError], so
/// callers treat server and on-device errors alike.
class VtopServerClient {
  VtopServerClient(
    this.settings, {
    http.Client? httpClient,
    this.timeout = const Duration(seconds: 60),
  }) : _http = httpClient ?? http.Client();

  final VtopServerSettings settings;
  final http.Client _http;
  final Duration timeout;

  /// POSTs [body] to `/v1/<path>` and returns the decoded JSON object as the
  /// server sent it (Rust serde shape) together with the response.
  Future<(Map<String, dynamic>, http.Response)> post(
    String path,
    Map<String, Object?> body, {
    Duration? timeout,
  }) async {
    final http.Response response;
    try {
      response = await _http
          .post(
            settings.endpoint(path),
            headers: settings.headers,
            body: jsonEncode(body),
          )
          .timeout(timeout ?? this.timeout);
    } on TimeoutException {
      throw const VtopError.networkError();
    } on SocketException {
      throw const VtopError.networkError();
    } on http.ClientException {
      throw const VtopError.networkError();
    }
    if (response.statusCode != 200) throw errorFor(response);
    final decoded = jsonDecode(utf8.decode(response.bodyBytes));
    if (decoded is! Map<String, dynamic>) {
      throw const VtopError.invalidResponse();
    }
    return (decoded, response);
  }

  static VtopError errorFor(http.Response response) {
    var code = '';
    var message = 'vtop-server returned HTTP ${response.statusCode}';
    try {
      final body = jsonDecode(utf8.decode(response.bodyBytes));
      if (body is Map && body['error'] is Map) {
        code = '${body['error']['code'] ?? ''}';
        message = '${body['error']['message'] ?? message}';
      }
    } on FormatException {
      // Not our JSON (e.g. a proxy error page); fall back to the status.
    }
    return errorForCode(code, message);
  }

  static VtopError errorForCode(String code, String message) {
    switch (code) {
      case 'session_expired':
        return const VtopError.sessionExpired();
      case 'invalid_credentials':
        return const VtopError.invalidCredentials();
      case 'authentication_failed':
      case 'otp_required':
        return VtopError.authenticationFailed(message);
      case 'invalid_input':
      case 'bad_request':
        return VtopError.configurationError(message);
      case 'unauthorized':
        return const VtopError.configurationError(
          'vtop-server rejected the API key (or needs one). Check it in Settings.',
        );
      case 'login_disabled':
        return const VtopError.configurationError(
          'This vtop-server does not accept logins. Turn off the server in Settings or enable login on it.',
        );
      case 'rate_limited':
        return const VtopError.vtopServerError(
          'vtop-server is rate limiting requests. Try again in a minute.',
        );
      case 'captcha_unavailable':
        return const VtopError.captchaRequired();
      case 'vtop_unreachable':
      case 'timeout':
        return const VtopError.networkError();
      case 'vtop_unparseable':
        return VtopError.parseError(message);
      default:
        return VtopError.vtopServerError(message);
    }
  }
}

/// [SessionState] in vtop-server's JSON shape.
Map<String, Object?> sessionToServerJson(SessionState session) => {
  'cookies': session.cookies,
  if (session.csrfToken != null) 'csrf_token': session.csrfToken,
  if (session.registrationNumber != null)
    'registration_number': session.registrationNumber,
  if (session.otpIssuedAt != null)
    'otp_issued_at': session.otpIssuedAt!.toInt(),
  if (session.loggedInAt != null) 'logged_in_at': session.loggedInAt!.toInt(),
};

/// A session vtop-server sent back.
SessionState sessionFromServerJson(Map<String, dynamic> json) {
  BigInt? number(Object? value) =>
      value is num ? BigInt.from(value.toInt()) : null;
  return SessionState(
    cookies: '${json['cookies'] ?? ''}',
    csrfToken: json['csrf_token'] as String?,
    registrationNumber: json['registration_number'] as String?,
    otpIssuedAt: number(json['otp_issued_at']),
    loggedInAt: number(json['logged_in_at']),
  );
}
