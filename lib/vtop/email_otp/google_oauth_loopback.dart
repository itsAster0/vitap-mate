import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:http/http.dart' as http;

const _maximumCredentialFileBytes = 64 * 1024;
const _googleAuthorizationEndpoint =
    'https://accounts.google.com/o/oauth2/v2/auth';
const _googleTokenEndpoint = 'https://oauth2.googleapis.com/token';

class GoogleDesktopOAuthCredentials {
  const GoogleDesktopOAuthCredentials({
    required this.clientId,
    required this.projectId,
    this.clientSecret,
  });

  final String clientId;
  final String projectId;
  final String? clientSecret;

  String get redactedClientId {
    final prefix = clientId.split('.').first;
    if (prefix.length <= 10) return clientId;
    return '${prefix.substring(0, 6)}...${prefix.substring(prefix.length - 4)}.apps.googleusercontent.com';
  }

  static GoogleDesktopOAuthCredentials parseBytes(Uint8List bytes) {
    if (bytes.isEmpty) {
      throw const GoogleDesktopOAuthException(
        'empty_file',
        'The selected credential file is empty.',
      );
    }
    if (bytes.length > _maximumCredentialFileBytes) {
      throw const GoogleDesktopOAuthException(
        'file_too_large',
        'The OAuth credential file must be smaller than 64 KB.',
      );
    }

    Object? decoded;
    try {
      decoded = jsonDecode(utf8.decode(bytes));
    } catch (_) {
      throw const GoogleDesktopOAuthException(
        'invalid_json',
        'This is not a valid Google OAuth JSON file.',
      );
    }
    if (decoded is! Map<String, dynamic>) {
      throw const GoogleDesktopOAuthException(
        'invalid_json',
        'This is not a valid Google OAuth credential file.',
      );
    }
    if (decoded.containsKey('web')) {
      throw const GoogleDesktopOAuthException(
        'wrong_client_type',
        'Create a Desktop app OAuth client, not a Web application client.',
      );
    }
    final installed = decoded['installed'];
    if (installed is! Map<String, dynamic>) {
      throw const GoogleDesktopOAuthException(
        'wrong_client_type',
        'Select the JSON downloaded for a Google Desktop app OAuth client.',
      );
    }

    final clientId = (installed['client_id'] as String?)?.trim() ?? '';
    if (!clientId.endsWith('.apps.googleusercontent.com') ||
        clientId.length <= '.apps.googleusercontent.com'.length) {
      throw const GoogleDesktopOAuthException(
        'invalid_client_id',
        'The credential file does not contain a valid Google OAuth client ID.',
      );
    }
    final secret = (installed['client_secret'] as String?)?.trim();
    final projectId = (installed['project_id'] as String?)?.trim();
    return GoogleDesktopOAuthCredentials(
      clientId: clientId,
      clientSecret: secret == null || secret.isEmpty ? null : secret,
      projectId: projectId == null || projectId.isEmpty
          ? 'Google Cloud project'
          : projectId,
    );
  }
}

class GoogleLoopbackOAuthTokens {
  const GoogleLoopbackOAuthTokens({
    required this.accessToken,
    required this.refreshToken,
    required this.expiresAt,
    required this.scopes,
    this.idToken,
  });

  final String accessToken;
  final String refreshToken;
  final DateTime expiresAt;
  final List<String> scopes;
  final String? idToken;
}

class GoogleDesktopOAuthException implements Exception {
  const GoogleDesktopOAuthException(this.code, this.message);

  final String code;
  final String message;

  @override
  String toString() => message;
}

typedef OAuthBrowserLauncher = Future<bool> Function(Uri uri);

class GoogleLoopbackOAuthCoordinator {
  GoogleLoopbackOAuthCoordinator({
    required http.Client httpClient,
    this.timeout = const Duration(minutes: 5),
    Uri? authorizationEndpoint,
    Uri? tokenEndpoint,
    Random? random,
  }) : _httpClient = httpClient,
       authorizationEndpoint =
           authorizationEndpoint ?? Uri.parse(_googleAuthorizationEndpoint),
       tokenEndpoint = tokenEndpoint ?? Uri.parse(_googleTokenEndpoint),
       _random = random ?? Random.secure();

  final http.Client _httpClient;
  final Duration timeout;
  final Uri authorizationEndpoint;
  final Uri tokenEndpoint;
  final Random _random;

  HttpServer? _activeServer;
  Completer<void>? _cancellation;

  bool get isActive => _activeServer != null;

  Future<void> cancel() async {
    final cancellation = _cancellation;
    if (cancellation != null && !cancellation.isCompleted) {
      cancellation.complete();
    }
    await _activeServer?.close(force: true);
    _activeServer = null;
  }

  Future<GoogleLoopbackOAuthTokens> authorize({
    required GoogleDesktopOAuthCredentials credentials,
    required List<String> scopes,
    required OAuthBrowserLauncher openBrowser,
    Future<void> Function()? beforeTokenExchange,
    void Function(String message)? onProgress,
  }) async {
    if (_activeServer != null) {
      throw const GoogleDesktopOAuthException(
        'already_running',
        'Google authorization is already in progress.',
      );
    }

    HttpServer server;
    try {
      server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    } catch (_) {
      throw const GoogleDesktopOAuthException(
        'localhost_unavailable',
        'Could not start the temporary localhost connection. Restart the app and try again.',
      );
    }
    _activeServer = server;
    _cancellation = Completer<void>();

    final redirectUri = Uri.parse(
      'http://127.0.0.1:${server.port}/oauth2callback',
    );
    final state = _randomUrlSafeString(32);
    final verifier = _randomUrlSafeString(64);
    final challenge = base64Url
        .encode(sha256.convert(ascii.encode(verifier)).bytes)
        .replaceAll('=', '');
    final callback = Completer<Uri>();
    late final StreamSubscription<HttpRequest> subscription;
    subscription = server.listen((request) async {
      if (request.uri.path != '/oauth2callback') {
        request.response.statusCode = HttpStatus.notFound;
        await request.response.close();
        return;
      }
      if (request.uri.queryParameters['state'] != state) {
        request.response.statusCode = HttpStatus.badRequest;
        request.response.headers.contentType = ContentType.html;
        request.response.write(
          _callbackHtml(
            'Authorization rejected',
            'The security check failed. Return to Vitap Mate and try again.',
          ),
        );
        await request.response.close();
        if (!callback.isCompleted) {
          callback.completeError(
            const GoogleDesktopOAuthException(
              'state_mismatch',
              'Google returned an invalid security state. Restart setup and try again.',
            ),
          );
        }
        return;
      }

      request.response.headers.contentType = ContentType.html;
      request.response.write(
        _callbackHtml(
          'Authorization received',
          'Return to Vitap Mate to finish setup. Keep the app open until it confirms Gmail is connected.',
        ),
      );
      await request.response.close();
      if (!callback.isCompleted) callback.complete(request.uri);
    });

    try {
      final authorizationUri = authorizationEndpoint.replace(
        queryParameters: {
          'client_id': credentials.clientId,
          'redirect_uri': redirectUri.toString(),
          'response_type': 'code',
          'scope': scopes.join(' '),
          'state': state,
          'code_challenge': challenge,
          'code_challenge_method': 'S256',
          'access_type': 'offline',
          'prompt': 'consent',
          'include_granted_scopes': 'true',
        },
      );
      final opened = await openBrowser(authorizationUri);
      if (!opened) {
        throw const GoogleDesktopOAuthException(
          'browser_launch_failed',
          'Could not open Google in your browser. Check that a browser is installed and try again.',
        );
      }

      final callbackUri = await Future.any<Uri>([
        callback.future,
        Future<Uri>.delayed(
          timeout,
          () => throw const GoogleDesktopOAuthException(
            'timeout',
            'Google authorization timed out. If the browser could not connect, return to the app and retry.',
          ),
        ),
        _cancellation!.future.then<Uri>(
          (_) => throw const GoogleDesktopOAuthException(
            'cancelled',
            'Google authorization was cancelled.',
          ),
        ),
      ]);

      onProgress?.call(
        'Browser authorization returned. Exchanging the Google code…',
      );

      final googleError = callbackUri.queryParameters['error'];
      if (googleError != null && googleError.isNotEmpty) {
        throw GoogleDesktopOAuthException(
          googleError,
          _authorizationErrorMessage(googleError),
        );
      }
      final code = callbackUri.queryParameters['code'];
      if (code == null || code.isEmpty) {
        throw const GoogleDesktopOAuthException(
          'missing_code',
          'Google did not return an authorization code. Try again.',
        );
      }

      if (beforeTokenExchange != null) {
        onProgress?.call(
          'Google approved access. Return to Vitap Mate to finish setup…',
        );
        await Future.any<void>([
          beforeTokenExchange(),
          _cancellation!.future.then<void>(
            (_) => throw const GoogleDesktopOAuthException(
              'cancelled',
              'Google authorization was cancelled.',
            ),
          ),
          Future<void>.delayed(
            timeout,
            () => throw const GoogleDesktopOAuthException(
              'foreground_timeout',
              'Google approved access, but Vitap Mate was not reopened in time. Return to the app and retry.',
            ),
          ),
        ]);
      }

      final body = <String, String>{
        'client_id': credentials.clientId,
        'code': code,
        'code_verifier': verifier,
        'grant_type': 'authorization_code',
        'redirect_uri': redirectUri.toString(),
      };
      if (credentials.clientSecret case final secret?) {
        body['client_secret'] = secret;
      }
      final response = await _exchangeCode(body);
      final decoded = _decodeResponse(response.body);
      if (response.statusCode != HttpStatus.ok) {
        final error = '${decoded['error'] ?? 'token_exchange_failed'}';
        throw GoogleDesktopOAuthException(error, _tokenErrorMessage(error));
      }

      final accessToken = '${decoded['access_token'] ?? ''}'.trim();
      final refreshToken = '${decoded['refresh_token'] ?? ''}'.trim();
      if (accessToken.isEmpty) {
        throw const GoogleDesktopOAuthException(
          'missing_access_token',
          'Google did not return an access token. Try again.',
        );
      }
      if (refreshToken.isEmpty) {
        throw const GoogleDesktopOAuthException(
          'missing_refresh_token',
          'Google did not return a refresh token. Remove the app from your Google Account permissions, then reconnect.',
        );
      }
      final expiresIn = decoded['expires_in'];
      final expiresInSeconds = expiresIn is int
          ? expiresIn
          : int.tryParse('$expiresIn') ?? 3600;
      final returnedScopes = '${decoded['scope'] ?? ''}'
          .split(RegExp(r'\s+'))
          .where((scope) => scope.isNotEmpty)
          .toList(growable: false);
      final effectiveScopes = returnedScopes.isEmpty ? scopes : returnedScopes;
      const gmailModifyScope = 'https://www.googleapis.com/auth/gmail.modify';
      if (scopes.contains(gmailModifyScope) &&
          !effectiveScopes.contains(gmailModifyScope)) {
        throw const GoogleDesktopOAuthException(
          'missing_gmail_scope',
          'Google sign-in completed, but Gmail access was not granted. Retry and approve the Gmail permission.',
        );
      }
      return GoogleLoopbackOAuthTokens(
        accessToken: accessToken,
        refreshToken: refreshToken,
        expiresAt: DateTime.now().toUtc().add(
          Duration(seconds: expiresInSeconds),
        ),
        scopes: effectiveScopes,
        idToken: (decoded['id_token'] as String?)?.trim(),
      );
    } finally {
      await subscription.cancel();
      await server.close(force: true);
      _activeServer = null;
      _cancellation = null;
    }
  }

  Future<http.Response> _exchangeCode(Map<String, String> body) async {
    for (var attempt = 0; attempt < 2; attempt++) {
      try {
        return await _httpClient
            .post(tokenEndpoint, body: body)
            .timeout(const Duration(seconds: 30));
      } on TimeoutException {
        // Retry once after the app has returned to the foreground.
      } on SocketException {
        // Retry once after the app has returned to the foreground.
      } on http.ClientException {
        // Retry once after the app has returned to the foreground.
      }
      if (attempt == 0) {
        await Future<void>.delayed(const Duration(milliseconds: 500));
      }
    }
    throw GoogleDesktopOAuthException(
      'token_network_failed',
      'Google approved access, but the app could not reach Google to finish setup. Return to Vitap Mate, check your connection, and retry.',
    );
  }

  String _randomUrlSafeString(int byteCount) {
    final bytes = List<int>.generate(byteCount, (_) => _random.nextInt(256));
    return base64Url.encode(bytes).replaceAll('=', '');
  }

  Map<String, dynamic> _decodeResponse(String body) {
    try {
      final decoded = jsonDecode(body);
      return decoded is Map<String, dynamic> ? decoded : const {};
    } catch (_) {
      return const {};
    }
  }

  String _authorizationErrorMessage(String error) => switch (error) {
    'access_denied' => 'Google authorization was cancelled.',
    'admin_policy_enforced' =>
      'Your VIT Google Workspace administrator blocked this Gmail permission.',
    'org_internal' =>
      'This OAuth project is restricted to another Google Workspace organization. Set the audience to External and add your VITAP email as a test user.',
    'invalid_scope' =>
      'Google rejected the Gmail permission request. Confirm that Gmail API is enabled and retry.',
    'redirect_uri_mismatch' =>
      'Google rejected the localhost redirect. Confirm that you imported a Desktop app OAuth JSON file.',
    _ => 'Google authorization failed ($error). Try again.',
  };

  String _tokenErrorMessage(String error) => switch (error) {
    'invalid_grant' =>
      'Google rejected or expired this grant. Testing-mode credentials commonly expire after seven days; reconnect and check your OAuth publishing status.',
    'invalid_client' =>
      'Google rejected these OAuth credentials. Download a fresh Desktop app JSON file and import it again.',
    'unauthorized_client' =>
      'This OAuth client is not allowed to request Gmail access. Create a Desktop app OAuth client and retry.',
    'invalid_scope' =>
      'Google rejected the Gmail permission. Enable Gmail API for the project and retry.',
    _ => 'Could not exchange the Google authorization code ($error).',
  };

  String _callbackHtml(String title, String message) =>
      '''
<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>$title</title><style>body{font-family:system-ui,sans-serif;max-width:36rem;margin:4rem auto;padding:0 1.5rem;line-height:1.5;color:#17202a}h1{font-size:1.6rem}</style></head>
<body><h1>$title</h1><p>$message</p></body></html>
''';
}
