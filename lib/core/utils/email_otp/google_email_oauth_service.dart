import 'dart:convert';
import 'dart:developer' show log;

import 'package:flutter_appauth/flutter_appauth.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;

const googleOauthClientId = String.fromEnvironment('GOOGLE_OAUTH_CLIENT_ID');
final googleOauthRedirectScheme =
    'com.googleusercontent.apps.${googleOauthClientId.replaceFirst('.apps.googleusercontent.com', '')}';
final googleOauthRedirectUrl = '$googleOauthRedirectScheme:/oauthredirect';

const googleEmailScopes = <String>[
  'openid',
  'email',
  'profile',
  'https://www.googleapis.com/auth/userinfo.email',
];
const gmailOauthScopes = <String>[
  'openid',
  'email',
  'profile',
  'https://www.googleapis.com/auth/gmail.readonly',
];
const _googleServiceConfiguration = AuthorizationServiceConfiguration(
  authorizationEndpoint: 'https://accounts.google.com/o/oauth2/v2/auth',
  tokenEndpoint: 'https://oauth2.googleapis.com/token',
);
const _oauthStorageKey = 'email_otp_oauth_session_v1';

final googleEmailOtpAuthServiceProvider = Provider<GoogleEmailOtpAuthService>((
  ref,
) {
  return GoogleEmailOtpAuthService(
    appAuth: const FlutterAppAuth(),
    storage: const FlutterSecureStorage(),
    httpClient: http.Client(),
  );
});

class EmailOtpSetupResult {
  const EmailOtpSetupResult({
    required this.success,
    required this.message,
    this.email,
  });

  final bool success;
  final String message;
  final String? email;
}

class LatestInfoEmail {
  const LatestInfoEmail({
    required this.receivedAt,
    required this.subject,
    required this.snippet,
    this.otp,
  });

  final DateTime receivedAt;
  final String subject;
  final String snippet;
  final String? otp;
}

class EmailOtpOAuthSession {
  const EmailOtpOAuthSession({
    required this.email,
    required this.accessToken,
    required this.refreshToken,
    required this.scopes,
    required this.accessTokenExpiryEpochMs,
  });

  final String email;
  final String accessToken;
  final String refreshToken;
  final List<String> scopes;
  final int accessTokenExpiryEpochMs;

  bool get hasGmailScope =>
      scopes.contains('https://www.googleapis.com/auth/gmail.readonly');

  bool get isExpired {
    final now = DateTime.now().toUtc().millisecondsSinceEpoch;
    return accessTokenExpiryEpochMs <= now + 30 * 1000;
  }

  Map<String, dynamic> toJson() {
    return {
      'email': email,
      'accessToken': accessToken,
      'refreshToken': refreshToken,
      'scopes': scopes,
      'accessTokenExpiryEpochMs': accessTokenExpiryEpochMs,
    };
  }

  static EmailOtpOAuthSession? fromJson(Map<String, dynamic> raw) {
    final email = (raw['email'] as String?)?.trim();
    final accessToken = (raw['accessToken'] as String?)?.trim();
    final refreshToken = (raw['refreshToken'] as String?)?.trim();
    final expiry = raw['accessTokenExpiryEpochMs'];
    final scopesRaw = raw['scopes'];
    if (email == null ||
        email.isEmpty ||
        accessToken == null ||
        accessToken.isEmpty ||
        refreshToken == null ||
        refreshToken.isEmpty ||
        expiry is! int ||
        scopesRaw is! List<dynamic>) {
      return null;
    }
    final scopes = scopesRaw.whereType<String>().toList(growable: false);
    return EmailOtpOAuthSession(
      email: email,
      accessToken: accessToken,
      refreshToken: refreshToken,
      scopes: scopes,
      accessTokenExpiryEpochMs: expiry,
    );
  }

  EmailOtpOAuthSession copyWith({
    String? accessToken,
    String? refreshToken,
    List<String>? scopes,
    int? accessTokenExpiryEpochMs,
  }) {
    return EmailOtpOAuthSession(
      email: email,
      accessToken: accessToken ?? this.accessToken,
      refreshToken: refreshToken ?? this.refreshToken,
      scopes: scopes ?? this.scopes,
      accessTokenExpiryEpochMs:
          accessTokenExpiryEpochMs ?? this.accessTokenExpiryEpochMs,
    );
  }
}

class GoogleEmailOtpAuthService {
  GoogleEmailOtpAuthService({
    required FlutterAppAuth appAuth,
    required FlutterSecureStorage storage,
    required http.Client httpClient,
  }) : _appAuth = appAuth,
       _storage = storage,
       _http = httpClient;

  final FlutterAppAuth _appAuth;
  final FlutterSecureStorage _storage;
  final http.Client _http;

  Future<EmailOtpOAuthSession?> loadSession() async {
    final raw = await _storage.read(key: _oauthStorageKey);
    if (raw == null || raw.isEmpty) return null;
    try {
      final json = jsonDecode(raw);
      if (json is! Map<String, dynamic>) return null;
      return EmailOtpOAuthSession.fromJson(json);
    } catch (_) {
      return null;
    }
  }

  Future<void> clearSession() async {
    await _storage.delete(key: _oauthStorageKey);
  }

  Future<bool> isReady() async {
    final session = await loadSession();
    if (session == null) return false;
    return session.hasGmailScope && session.refreshToken.isNotEmpty;
  }

  Future<EmailOtpSetupResult> setupIdentityThenGmail({
    required String expectedUsername,
  }) async {
    final identity = await setupIdentityStep(
      expectedUsername: expectedUsername,
    );
    if (!identity.success || identity.email == null) return identity;
    return setupGmailTokenStep(email: identity.email!);
  }

  Future<EmailOtpSetupResult> setupIdentityStep({
    required String expectedUsername,
  }) async {
    if (googleOauthClientId.isEmpty) {
      return const EmailOtpSetupResult(
        success: false,
        message:
            'Missing GOOGLE_OAUTH_CLIENT_ID. Pass it with --dart-define or --dart-define-from-file before using Google sign-in.',
      );
    }
    try {
      log(
        'Starting identity OAuth request with scopes: ${googleEmailScopes.join(', ')}',
        name: 'email_otp.oauth',
      );
      final identityTokens = await _appAuth.authorizeAndExchangeCode(
        AuthorizationTokenRequest(
          googleOauthClientId,
          googleOauthRedirectUrl,
          serviceConfiguration: _googleServiceConfiguration,
          scopes: googleEmailScopes,
          promptValues: const ['consent'],
          additionalParameters: const {'access_type': 'offline'},
        ),
      );
      if (identityTokens.accessToken == null) {
        return const EmailOtpSetupResult(
          success: false,
          message: 'Google sign-in was cancelled.',
        );
      }

      final email = await _resolveAccountEmail(
        accessToken: identityTokens.accessToken!,
        idToken: identityTokens.idToken,
      );
      if (email == null || email.isEmpty) {
        return const EmailOtpSetupResult(
          success: false,
          message: 'Could not read your Google account email.',
        );
      }
      if (!email.toLowerCase().endsWith('@vitapstudent.ac.in')) {
        return EmailOtpSetupResult(
          success: false,
          message: 'Use your @vitapstudent.ac.in email for OTP autofetch.',
          email: email,
        );
      }

      final fromEmail = _usernameFromCollegeEmail(email);
      final fromClient = expectedUsername.trim().toLowerCase();
      if (fromEmail.isEmpty || fromEmail != fromClient) {
        return EmailOtpSetupResult(
          success: false,
          message:
              'Username mismatch: VTOP username and email username do not match.',
          email: email,
        );
      }
      return EmailOtpSetupResult(
        success: true,
        message: 'Email verified. Continue to token step.',
        email: email,
      );
    } catch (error, stackTrace) {
      log(
        'Identity step failed',
        name: 'email_otp.oauth',
        error: error,
        stackTrace: stackTrace,
      );
      return EmailOtpSetupResult(
        success: false,
        message: _friendlyOauthError(error),
      );
    }
  }

  Future<EmailOtpSetupResult> setupGmailTokenStep({
    required String email,
  }) async {
    if (googleOauthClientId.isEmpty) {
      return const EmailOtpSetupResult(
        success: false,
        message:
            'Missing GOOGLE_OAUTH_CLIENT_ID. Pass it with --dart-define or --dart-define-from-file before using Google sign-in.',
      );
    }
    try {
      log(
        'Starting Gmail token OAuth request for $email with scopes: ${gmailOauthScopes.join(', ')}',
        name: 'email_otp.oauth',
      );
      final gmailTokens = await _appAuth.authorizeAndExchangeCode(
        AuthorizationTokenRequest(
          googleOauthClientId,
          googleOauthRedirectUrl,
          serviceConfiguration: _googleServiceConfiguration,
          scopes: gmailOauthScopes,
          loginHint: email,
          promptValues: const ['consent'],
          additionalParameters: const {
            'access_type': 'offline',
            'include_granted_scopes': 'true',
          },
        ),
      );
      if (gmailTokens.accessToken == null) {
        return const EmailOtpSetupResult(
          success: false,
          message: 'Gmail permission setup was cancelled.',
        );
      }
      final refreshToken = (gmailTokens.refreshToken ?? '').trim();
      if (refreshToken.isEmpty) {
        return const EmailOtpSetupResult(
          success: false,
          message: 'Could not get a refresh token for Gmail access.',
        );
      }
      final expiry =
          gmailTokens.accessTokenExpirationDateTime?.toUtc() ??
          DateTime.now().toUtc().add(const Duration(minutes: 50));
      final scopes = <String>{...gmailOauthScopes}.toList(growable: false);
      final session = EmailOtpOAuthSession(
        email: email,
        accessToken: gmailTokens.accessToken!,
        refreshToken: refreshToken,
        scopes: scopes,
        accessTokenExpiryEpochMs: expiry.millisecondsSinceEpoch,
      );
      await _storage.write(
        key: _oauthStorageKey,
        value: jsonEncode(session.toJson()),
      );
      return EmailOtpSetupResult(
        success: true,
        message: 'Email OTP autofetch is enabled.',
        email: email,
      );
    } catch (error, stackTrace) {
      log(
        'Token step failed',
        name: 'email_otp.oauth',
        error: error,
        stackTrace: stackTrace,
      );
      return EmailOtpSetupResult(
        success: false,
        message: _friendlyOauthError(error),
      );
    }
  }

  Future<EmailOtpOAuthSession?> refreshIfNeeded() async {
    final session = await loadSession();
    if (session == null) return null;
    if (!session.isExpired) return session;
    final token = await _appAuth.token(
      TokenRequest(
        googleOauthClientId,
        googleOauthRedirectUrl,
        serviceConfiguration: _googleServiceConfiguration,
        refreshToken: session.refreshToken,
        scopes: gmailOauthScopes,
      ),
    );
    if (token.accessToken == null) {
      await clearSession();
      return null;
    }
    final refreshed = session.copyWith(
      accessToken: token.accessToken,
      refreshToken: (token.refreshToken ?? session.refreshToken).trim(),
      accessTokenExpiryEpochMs:
          (token.accessTokenExpirationDateTime?.toUtc() ??
                  DateTime.now().toUtc().add(const Duration(minutes: 50)))
              .millisecondsSinceEpoch,
    );
    await _storage.write(
      key: _oauthStorageKey,
      value: jsonEncode(refreshed.toJson()),
    );
    return refreshed;
  }

  Future<String?> fetchLatestOtpSince({required DateTime sinceUtc}) async {
    final session = await refreshIfNeeded();
    if (session == null) return null;

    final listResponse = await _http.get(
      Uri.parse(
        'https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults=1&q=from:info1@vitap.ac.in',
      ),
      headers: {'Authorization': 'Bearer ${session.accessToken}'},
    );
    if (listResponse.statusCode == 401 || listResponse.statusCode == 403) {
      await clearSession();
      throw StateError('Gmail authorization expired.');
    }
    if (listResponse.statusCode != 200) {
      throw StateError(
        'Unable to read Gmail inbox (status ${listResponse.statusCode}).',
      );
    }

    final message = await _fetchFirstListedMessage(
      listResponse.body,
      session.accessToken,
    );
    if (message == null) return null;

    final internalDateMs = int.tryParse('${message['internalDate']}') ?? 0;
    if (internalDateMs <= 0) return null;
    final timestamp = DateTime.fromMillisecondsSinceEpoch(
      internalDateMs,
      isUtc: true,
    );
    if (timestamp.isBefore(sinceUtc)) return null;
    if (!_isFromVtopOtpSender(message)) return null;

    final combinedText = _extractMessageText(message);
    final match = RegExp(r'(?<!\d)(\d{6})(?!\d)').firstMatch(combinedText);
    return match?.group(1);
  }

  Future<LatestInfoEmail?> fetchLatestInfoEmail() async {
    final session = await refreshIfNeeded();
    if (session == null) {
      throw StateError('Email OTP OAuth is not connected.');
    }

    final listResponse = await _http.get(
      Uri.parse(
        'https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults=1&q=from:info1@vitap.ac.in',
      ),
      headers: {'Authorization': 'Bearer ${session.accessToken}'},
    );
    if (listResponse.statusCode == 401 || listResponse.statusCode == 403) {
      await clearSession();
      throw StateError('Gmail authorization expired.');
    }
    if (listResponse.statusCode != 200) {
      throw StateError(
        'Unable to read Gmail inbox (status ${listResponse.statusCode}).',
      );
    }

    final latest = await _fetchFirstListedMessage(
      listResponse.body,
      session.accessToken,
    );
    if (latest == null || !_isFromVtopOtpSender(latest)) return null;

    final internalDateMs = int.tryParse('${latest['internalDate']}') ?? 0;
    final receivedAt = DateTime.fromMillisecondsSinceEpoch(
      internalDateMs,
      isUtc: true,
    );
    final text = _extractMessageText(latest);
    final collapsed = text.replaceAll(RegExp(r'\s+'), ' ').trim();
    final snippet = collapsed.length > 500
        ? '${collapsed.substring(0, 500)}...'
        : collapsed;
    final otp = RegExp(r'(?<!\d)(\d{6})(?!\d)').firstMatch(text)?.group(1);

    return LatestInfoEmail(
      receivedAt: receivedAt,
      subject: _messageHeader(latest, 'subject') ?? '(no subject)',
      snippet: snippet.isEmpty ? '(empty message)' : snippet,
      otp: otp,
    );
  }

  Future<Map<String, dynamic>?> _fetchFirstListedMessage(
    String listResponseBody,
    String accessToken,
  ) async {
    final raw = jsonDecode(listResponseBody) as Map<String, dynamic>;
    final messages = raw['messages'];
    if (messages is! List<dynamic> || messages.isEmpty) return null;
    final first = messages.first;
    if (first is! Map<String, dynamic>) return null;
    final id = first['id'] as String?;
    if (id == null || id.isEmpty) return null;

    final res = await _http.get(
      Uri.parse(
        'https://gmail.googleapis.com/gmail/v1/users/me/messages/$id?format=full',
      ),
      headers: {'Authorization': 'Bearer $accessToken'},
    );
    if (res.statusCode != 200) return null;
    final message = jsonDecode(res.body);
    if (message is! Map<String, dynamic>) return null;
    return message;
  }

  Future<String?> _resolveAccountEmail({
    required String accessToken,
    String? idToken,
  }) async {
    final byIdToken = _emailFromIdToken(idToken);
    if (byIdToken != null && byIdToken.isNotEmpty) return byIdToken;

    final response = await _http.get(
      Uri.parse('https://openidconnect.googleapis.com/v1/userinfo'),
      headers: {'Authorization': 'Bearer $accessToken'},
    );
    if (response.statusCode != 200) return null;
    final json = jsonDecode(response.body);
    if (json is! Map<String, dynamic>) return null;
    return (json['email'] as String?)?.trim();
  }

  String? _emailFromIdToken(String? idToken) {
    if (idToken == null || idToken.isEmpty) return null;
    final parts = idToken.split('.');
    if (parts.length < 2) return null;
    final payload = parts[1];
    final normalized = base64Url.normalize(payload);
    try {
      final data = utf8.decode(base64Url.decode(normalized));
      final json = jsonDecode(data);
      if (json is! Map<String, dynamic>) return null;
      return (json['email'] as String?)?.trim();
    } catch (_) {
      return null;
    }
  }

  String _usernameFromCollegeEmail(String email) {
    final localPart = email.split('@').first;
    final parts = localPart
        .split('.')
        .where((part) => part.isNotEmpty)
        .toList(growable: false);
    if (parts.isEmpty) return '';
    return parts.last.toLowerCase();
  }

  bool _isFromVtopOtpSender(Map<String, dynamic> message) {
    final payload = message['payload'];
    if (payload is! Map<String, dynamic>) return false;
    final headers = payload['headers'];
    if (headers is! List<dynamic>) return false;
    for (final item in headers) {
      if (item is! Map<String, dynamic>) continue;
      final name = '${item['name']}'.toLowerCase();
      if (name != 'from') continue;
      final value = '${item['value']}'.toLowerCase();
      if (value.contains('info1@vitap.ac.in')) return true;
    }
    return false;
  }

  String? _messageHeader(Map<String, dynamic> message, String headerName) {
    final payload = message['payload'];
    if (payload is! Map<String, dynamic>) return null;
    final headers = payload['headers'];
    if (headers is! List<dynamic>) return null;
    for (final item in headers) {
      if (item is! Map<String, dynamic>) continue;
      final name = '${item['name']}'.toLowerCase();
      if (name == headerName.toLowerCase()) {
        return '${item['value']}'.trim();
      }
    }
    return null;
  }

  String _extractMessageText(Map<String, dynamic> message) {
    final buffer = StringBuffer();
    final snippet = message['snippet'];
    if (snippet is String && snippet.isNotEmpty) {
      buffer.writeln(snippet);
    }
    final payload = message['payload'];
    if (payload is Map<String, dynamic>) {
      _appendPayloadText(payload, buffer);
    }
    return _decodeQuotedPrintable(buffer.toString());
  }

  void _appendPayloadText(Map<String, dynamic> payload, StringBuffer buffer) {
    final body = payload['body'];
    if (body is Map<String, dynamic>) {
      final encoded = body['data'];
      if (encoded is String && encoded.isNotEmpty) {
        final text = _decodeBase64Url(encoded);
        if (text.isNotEmpty) {
          buffer.writeln(text);
        }
      }
    }
    final parts = payload['parts'];
    if (parts is! List<dynamic>) return;
    for (final part in parts) {
      if (part is! Map<String, dynamic>) continue;
      _appendPayloadText(part, buffer);
    }
  }

  String _decodeBase64Url(String value) {
    final normalized = base64Url.normalize(value);
    try {
      return utf8.decode(base64Url.decode(normalized), allowMalformed: true);
    } catch (_) {
      return '';
    }
  }

  String _decodeQuotedPrintable(String input) {
    final softBreakReplaced = input.replaceAll(RegExp(r'=\r?\n'), '');
    return softBreakReplaced.replaceAllMapped(RegExp(r'=([0-9A-Fa-f]{2})'), (
      match,
    ) {
      final hex = match.group(1);
      if (hex == null) return '';
      final code = int.tryParse(hex, radix: 16);
      if (code == null) return '';
      return String.fromCharCode(code);
    });
  }

  String _friendlyOauthError(Object error) {
    final text = error.toString().toLowerCase();
    if (text.contains('user cancelled') || text.contains('cancelled')) {
      return 'Google sign-in was cancelled.';
    }
    if (text.contains('null_intent') || text.contains('failed to authorize')) {
      return 'Could not launch Google authorization. Please try again.';
    }
    return 'OAuth setup failed. Please try again.';
  }
}
