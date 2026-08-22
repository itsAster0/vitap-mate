import 'dart:async';
import 'dart:convert';
import 'dart:developer' show log;
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter_appauth/flutter_appauth.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/utils/email_otp/google_oauth_loopback.dart';
import 'package:vitapmate/core/utils/featureflags/feature_flags.dart';

part 'google_email_oauth_service.g.dart';

const googleOauthClientId = String.fromEnvironment('GOOGLE_OAUTH_CLIENT_ID');
bool get sharedGoogleOAuthEnabled => googleOauthClientId.trim().isNotEmpty;
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
  'https://www.googleapis.com/auth/gmail.modify',
];
const _googleServiceConfiguration = AuthorizationServiceConfiguration(
  authorizationEndpoint: 'https://accounts.google.com/o/oauth2/v2/auth',
  tokenEndpoint: 'https://oauth2.googleapis.com/token',
);
const _oauthStorageKey = 'email_otp_oauth_session_v1';

@Riverpod(keepAlive: true)
GoogleEmailOtpAuthService googleEmailOtpAuthService(Ref ref) {
  final httpClient = http.Client();
  return GoogleEmailOtpAuthService(
    appAuth: const FlutterAppAuth(),
    storage: const FlutterSecureStorage(),
    httpClient: httpClient,
    loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: httpClient),
  );
}

@Riverpod(keepAlive: true)
Future<bool> emailOtpReady(Ref ref) {
  return ref.watch(googleEmailOtpAuthServiceProvider).isReady();
}

@Riverpod(keepAlive: true)
Future<bool> emailOtpSetupNeeded(Ref ref) async {
  final flags = await ref.watch(featureFlagsControllerProvider.future);
  if (!await flags.isEnabled('2fa-email')) return false;
  return !await ref.watch(emailOtpReadyProvider.future);
}

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

enum EmailOtpAuthSource { sharedBuiltIn, personalByok }

class EmailOtpOAuthSession {
  const EmailOtpOAuthSession({
    required this.email,
    required this.accessToken,
    required this.refreshToken,
    required this.scopes,
    required this.accessTokenExpiryEpochMs,
    this.schemaVersion = 2,
    this.authSource = EmailOtpAuthSource.sharedBuiltIn,
    this.oauthClientId,
    this.oauthClientSecret,
  });

  final String email;
  final String accessToken;
  final String refreshToken;
  final List<String> scopes;
  final int accessTokenExpiryEpochMs;
  final int schemaVersion;
  final EmailOtpAuthSource authSource;
  final String? oauthClientId;
  final String? oauthClientSecret;

  String get authSourceLabel => switch (authSource) {
    EmailOtpAuthSource.personalByok => 'Personal BYOK',
    EmailOtpAuthSource.sharedBuiltIn => 'Shared fallback',
  };

  bool get hasGmailScope =>
      scopes.contains('https://www.googleapis.com/auth/gmail.modify');

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
      'schemaVersion': schemaVersion,
      'authSource': authSource.name,
      if (oauthClientId != null) 'oauthClientId': oauthClientId,
      if (oauthClientSecret != null) 'oauthClientSecret': oauthClientSecret,
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
    final sourceName = raw['authSource'] as String?;
    final authSource = EmailOtpAuthSource.values.firstWhere(
      (value) => value.name == sourceName,
      orElse: () => EmailOtpAuthSource.sharedBuiltIn,
    );
    return EmailOtpOAuthSession(
      email: email,
      accessToken: accessToken,
      refreshToken: refreshToken,
      scopes: scopes,
      accessTokenExpiryEpochMs: expiry,
      schemaVersion: raw['schemaVersion'] is int
          ? raw['schemaVersion'] as int
          : 1,
      authSource: authSource,
      oauthClientId: (raw['oauthClientId'] as String?)?.trim(),
      oauthClientSecret: (raw['oauthClientSecret'] as String?)?.trim(),
    );
  }

  EmailOtpOAuthSession copyWith({
    String? accessToken,
    String? refreshToken,
    List<String>? scopes,
    int? accessTokenExpiryEpochMs,
    int? schemaVersion,
    EmailOtpAuthSource? authSource,
    String? oauthClientId,
    String? oauthClientSecret,
  }) {
    return EmailOtpOAuthSession(
      email: email,
      accessToken: accessToken ?? this.accessToken,
      refreshToken: refreshToken ?? this.refreshToken,
      scopes: scopes ?? this.scopes,
      accessTokenExpiryEpochMs:
          accessTokenExpiryEpochMs ?? this.accessTokenExpiryEpochMs,
      schemaVersion: schemaVersion ?? this.schemaVersion,
      authSource: authSource ?? this.authSource,
      oauthClientId: oauthClientId ?? this.oauthClientId,
      oauthClientSecret: oauthClientSecret ?? this.oauthClientSecret,
    );
  }
}

class GoogleEmailOtpAuthService {
  GoogleEmailOtpAuthService({
    required FlutterAppAuth appAuth,
    required FlutterSecureStorage storage,
    required http.Client httpClient,
    required GoogleLoopbackOAuthCoordinator loopbackOAuth,
  }) : _appAuth = appAuth,
       _storage = storage,
       _http = httpClient,
       _loopbackOAuth = loopbackOAuth;

  final FlutterAppAuth _appAuth;
  final FlutterSecureStorage _storage;
  final http.Client _http;
  final GoogleLoopbackOAuthCoordinator _loopbackOAuth;

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
    await _loopbackOAuth.cancel();
    await _storage.delete(key: _oauthStorageKey);
  }

  Future<void> cancelByokSetup() => _loopbackOAuth.cancel();

  GoogleDesktopOAuthCredentials parseByokCredentials(List<int> bytes) {
    return GoogleDesktopOAuthCredentials.parseBytes(
      bytes is Uint8List ? bytes : Uint8List.fromList(bytes),
    );
  }

  Future<bool> isReady() async {
    final session = await loadSession();
    if (session == null) return false;
    if (session.authSource == EmailOtpAuthSource.sharedBuiltIn &&
        !sharedGoogleOAuthEnabled) {
      return false;
    }
    return session.hasGmailScope && session.refreshToken.isNotEmpty;
  }

  Stream<EmailOtpOAuthSession> pollForPersonalSession({
    required String clientId,
    EmailOtpOAuthSession? previousSession,
    Duration interval = const Duration(seconds: 1),
    Duration timeout = const Duration(minutes: 5),
  }) async* {
    final elapsed = Stopwatch()..start();
    while (elapsed.elapsed < timeout) {
      final session = await loadSession();
      if (session != null &&
          session.authSource == EmailOtpAuthSource.personalByok &&
          session.oauthClientId == clientId &&
          session.refreshToken.isNotEmpty &&
          session.hasGmailScope &&
          !_sameAuthorization(session, previousSession)) {
        yield session;
        return;
      }
      await Future<void>.delayed(interval);
    }
  }

  bool _sameAuthorization(
    EmailOtpOAuthSession session,
    EmailOtpOAuthSession? previous,
  ) =>
      previous != null &&
      session.authSource == previous.authSource &&
      session.oauthClientId == previous.oauthClientId &&
      session.refreshToken == previous.refreshToken &&
      session.accessToken == previous.accessToken &&
      session.accessTokenExpiryEpochMs == previous.accessTokenExpiryEpochMs;

  Future<EmailOtpSetupResult> setupByok({
    required GoogleDesktopOAuthCredentials credentials,
    required String expectedUsername,
    required OAuthBrowserLauncher openBrowser,
    Future<void> Function()? beforeTokenExchange,
    void Function(String message)? onProgress,
  }) async {
    var stage = 'waiting for the localhost callback';
    try {
      final tokens = await _loopbackOAuth.authorize(
        credentials: credentials,
        scopes: gmailOauthScopes,
        openBrowser: openBrowser,
        beforeTokenExchange: beforeTokenExchange,
        onProgress: onProgress,
      );
      stage = 'verifying the Google account';
      onProgress?.call('Google access received. Verifying your college email…');
      final email = await _resolveAccountEmail(
        accessToken: tokens.accessToken,
        idToken: tokens.idToken,
      );
      final validationMessage = _validateCollegeAccount(
        email: email,
        expectedUsername: expectedUsername,
      );
      if (validationMessage != null) {
        await _revokeToken(tokens.refreshToken);
        return EmailOtpSetupResult(
          success: false,
          message: validationMessage,
          email: email,
        );
      }

      stage = 'checking Gmail access';
      onProgress?.call('College account verified. Checking Gmail access…');
      try {
        await _verifyGmailAccess(tokens.accessToken);
      } on GoogleDesktopOAuthException {
        await _revokeToken(tokens.refreshToken);
        rethrow;
      }

      final session = EmailOtpOAuthSession(
        email: email!,
        accessToken: tokens.accessToken,
        refreshToken: tokens.refreshToken,
        scopes: tokens.scopes,
        accessTokenExpiryEpochMs: tokens.expiresAt.millisecondsSinceEpoch,
        authSource: EmailOtpAuthSource.personalByok,
        oauthClientId: credentials.clientId,
        oauthClientSecret: credentials.clientSecret,
      );
      stage = 'saving access securely on this device';
      onProgress?.call('College account verified. Saving Gmail access…');
      await _storage.write(
        key: _oauthStorageKey,
        value: jsonEncode(session.toJson()),
      );
      final saved = await loadSession();
      if (saved == null ||
          saved.authSource != EmailOtpAuthSource.personalByok ||
          saved.oauthClientId != credentials.clientId ||
          saved.refreshToken != tokens.refreshToken) {
        throw const GoogleDesktopOAuthException(
          'secure_storage_failed',
          'Google access was approved, but it could not be saved securely on this device. Unlock the device, restart Vitap Mate, and retry.',
        );
      }
      return EmailOtpSetupResult(
        success: true,
        message: 'Personal Gmail access is connected.',
        email: email,
      );
    } on GoogleDesktopOAuthException catch (error) {
      return EmailOtpSetupResult(success: false, message: error.message);
    } catch (error, stackTrace) {
      log(
        'Personal OAuth setup failed while $stage',
        name: 'email_otp.oauth',
        error: error,
        stackTrace: stackTrace,
      );
      debugPrint('Gmail BYOK failed while $stage (${error.runtimeType}).');
      return EmailOtpSetupResult(
        success: false,
        message: _byokFailureMessage(stage, error),
      );
    }
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
        message: 'Shared Google access is not configured in this build.',
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
      final validationMessage = _validateCollegeAccount(
        email: email,
        expectedUsername: expectedUsername,
      );
      if (validationMessage != null) {
        return EmailOtpSetupResult(
          success: false,
          message: validationMessage,
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
        message: 'Shared Google access is not configured in this build.',
      );
    }
    try {
      log(
        'Starting shared Gmail token OAuth request with scopes: ${gmailOauthScopes.join(', ')}',
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
      final grantedScopes = gmailTokens.scopes?.isNotEmpty == true
          ? gmailTokens.scopes!
          : gmailOauthScopes;
      if (!grantedScopes.contains(
        'https://www.googleapis.com/auth/gmail.modify',
      )) {
        return const EmailOtpSetupResult(
          success: false,
          message:
              'Google sign-in completed, but Gmail access was not granted. Retry and approve the Gmail permission.',
        );
      }
      try {
        await _verifyGmailAccess(gmailTokens.accessToken!);
      } on GoogleDesktopOAuthException catch (error) {
        return EmailOtpSetupResult(success: false, message: error.message);
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
      final scopes = <String>{...grantedScopes}.toList(growable: false);
      final session = EmailOtpOAuthSession(
        email: email,
        accessToken: gmailTokens.accessToken!,
        refreshToken: refreshToken,
        scopes: scopes,
        accessTokenExpiryEpochMs: expiry.millisecondsSinceEpoch,
        authSource: EmailOtpAuthSource.sharedBuiltIn,
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
    if (session.authSource == EmailOtpAuthSource.personalByok) {
      return _refreshByokSession(session);
    }
    if (!sharedGoogleOAuthEnabled) {
      await clearSession();
      return null;
    }
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

  Future<EmailOtpOAuthSession?> _refreshByokSession(
    EmailOtpOAuthSession session,
  ) async {
    final clientId = session.oauthClientId?.trim() ?? '';
    if (clientId.isEmpty) {
      await clearSession();
      throw StateError(
        'Personal OAuth credentials are missing. Import the Desktop OAuth JSON again.',
      );
    }
    final body = <String, String>{
      'client_id': clientId,
      'refresh_token': session.refreshToken,
      'grant_type': 'refresh_token',
    };
    final secret = session.oauthClientSecret?.trim();
    if (secret != null && secret.isNotEmpty) {
      body['client_secret'] = secret;
    }
    final response = await _http.post(
      Uri.parse('https://oauth2.googleapis.com/token'),
      body: body,
    );
    Map<String, dynamic> decoded = const {};
    try {
      final value = jsonDecode(response.body);
      if (value is Map<String, dynamic>) decoded = value;
    } catch (_) {}
    if (response.statusCode != 200) {
      final error = '${decoded['error'] ?? 'refresh_failed'}';
      if (error == 'invalid_grant') {
        await clearSession();
        throw StateError(
          'Google access expired or was revoked. Testing-mode credentials commonly expire after seven days; reconnect and check the OAuth publishing status.',
        );
      }
      throw StateError('Could not refresh personal Google access ($error).');
    }
    final accessToken = '${decoded['access_token'] ?? ''}'.trim();
    if (accessToken.isEmpty) {
      await clearSession();
      return null;
    }
    final expiresIn = decoded['expires_in'];
    final seconds = expiresIn is int
        ? expiresIn
        : int.tryParse('$expiresIn') ?? 3600;
    final refreshed = session.copyWith(
      accessToken: accessToken,
      refreshToken: '${decoded['refresh_token'] ?? session.refreshToken}'
          .trim(),
      accessTokenExpiryEpochMs: DateTime.now()
          .toUtc()
          .add(Duration(seconds: seconds))
          .millisecondsSinceEpoch,
    );
    await _storage.write(
      key: _oauthStorageKey,
      value: jsonEncode(refreshed.toJson()),
    );
    return refreshed;
  }

  Future<String?> fetchLatestOtpSince({
    required DateTime sinceUtc,
    bool deleteAfterReading = true,
  }) async {
    final session = await refreshIfNeeded();
    if (session == null) return null;

    final listResponse = await _http.get(
      Uri.parse(
        'https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults=1&q=from:noreply.sdc@vitap.ac.in',
      ),
      headers: {'Authorization': 'Bearer ${session.accessToken}'},
    );
    await _throwForGmailReadFailure(listResponse);
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
    final otp = match?.group(1);
    if (otp != null) {
      await _handleReadOtpMessage(
        message,
        session.accessToken,
        deleteAfterReading: deleteAfterReading,
      );
    }
    return otp;
  }

  Future<LatestInfoEmail?> fetchLatestInfoEmail({
    bool deleteAfterReading = true,
  }) async {
    final session = await refreshIfNeeded();
    if (session == null) {
      throw StateError('Email OTP OAuth is not connected.');
    }

    final listResponse = await _http.get(
      Uri.parse(
        'https://gmail.googleapis.com/gmail/v1/users/me/messages?maxResults=1&q=from:noreply.sdc@vitap.ac.in',
      ),
      headers: {'Authorization': 'Bearer ${session.accessToken}'},
    );
    await _throwForGmailReadFailure(listResponse);
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
    if (otp != null) {
      await _handleReadOtpMessage(
        latest,
        session.accessToken,
        deleteAfterReading: deleteAfterReading,
      );
    }

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

  Future<void> _markMessageAsRead(
    Map<String, dynamic> message,
    String accessToken,
  ) async {
    final id = message['id'] as String?;
    if (id == null || id.isEmpty) return;
    try {
      final response = await _http.post(
        Uri.parse(
          'https://gmail.googleapis.com/gmail/v1/users/me/messages/$id/modify',
        ),
        headers: {
          'Authorization': 'Bearer $accessToken',
          'Content-Type': 'application/json',
        },
        body: jsonEncode({
          'removeLabelIds': ['UNREAD'],
        }),
      );
      if (response.statusCode != 200) {
        log(
          'Failed to mark OTP email as read (status ${response.statusCode}).',
          name: 'email_otp.gmail',
        );
      }
    } catch (error, stackTrace) {
      log(
        'Failed to mark OTP email as read',
        name: 'email_otp.gmail',
        error: error,
        stackTrace: stackTrace,
      );
    }
  }

  Future<void> _handleReadOtpMessage(
    Map<String, dynamic> message,
    String accessToken, {
    required bool deleteAfterReading,
  }) async {
    if (!deleteAfterReading) {
      await _markMessageAsRead(message, accessToken);
      return;
    }

    final id = message['id'] as String?;
    if (id == null || id.isEmpty) return;
    try {
      final response = await _http.post(
        Uri.parse(
          'https://gmail.googleapis.com/gmail/v1/users/me/messages/$id/trash',
        ),
        headers: {'Authorization': 'Bearer $accessToken'},
      );
      if (response.statusCode != 200) {
        log(
          'Failed to move OTP email to Trash (status ${response.statusCode}).',
          name: 'email_otp.gmail',
        );
      }
    } catch (error, stackTrace) {
      log(
        'Failed to move OTP email to Trash',
        name: 'email_otp.gmail',
        error: error,
        stackTrace: stackTrace,
      );
    }
  }

  Future<void> _verifyGmailAccess(String accessToken) async {
    final response = await _http
        .get(
          Uri.parse('https://gmail.googleapis.com/gmail/v1/users/me/profile'),
          headers: {'Authorization': 'Bearer $accessToken'},
        )
        .timeout(const Duration(seconds: 20));
    if (response.statusCode == 200) return;
    throw GoogleDesktopOAuthException(
      'gmail_access_denied',
      _gmailAccessFailureMessage(response),
    );
  }

  Future<void> _throwForGmailReadFailure(http.Response response) async {
    if (response.statusCode == 200) return;
    if (response.statusCode == 401) {
      await clearSession();
      throw StateError(
        'Google authorization expired or was revoked. Reconnect Gmail.',
      );
    }
    if (response.statusCode == 403) {
      final message = _gmailAccessFailureMessage(response);
      final normalized = response.body.toLowerCase();
      final apiCanBeEnabledWithoutReconnecting =
          normalized.contains('accessnotconfigured') ||
          normalized.contains('service_disabled') ||
          normalized.contains('api has not been used') ||
          normalized.contains('gmail api has not been used');
      if (!apiCanBeEnabledWithoutReconnecting) {
        await clearSession();
      }
      throw StateError(message);
    }
  }

  String _gmailAccessFailureMessage(http.Response response) {
    final body = response.body.toLowerCase();
    if (response.statusCode == 401) {
      return 'Google authorization expired or was revoked. Reconnect Gmail.';
    }
    if (body.contains('accessnotconfigured') ||
        body.contains('service_disabled') ||
        body.contains('api has not been used') ||
        body.contains('gmail api has not been used')) {
      return 'The Gmail API is not enabled for this Google Cloud project. Enable Gmail API, wait a minute, then retry.';
    }
    if (body.contains('admin_policy_enforced') ||
        body.contains('domainpolicy') ||
        body.contains('workspace') ||
        body.contains('administrator')) {
      return 'VIT Google Workspace policy blocked Gmail access. Contact the Workspace administrator or use another allowed OAuth project.';
    }
    if (body.contains('mailservicenotenabled') ||
        body.contains('mail service not enabled') ||
        body.contains('gmail service has not been enabled') ||
        body.contains('not a gmail user') ||
        body.contains('failed_precondition')) {
      return 'Gmail is not enabled for this Google account. Open Gmail once or ask the VIT Workspace administrator to enable Gmail, then retry.';
    }
    if (body.contains('insufficientpermissions') ||
        body.contains('access_token_scope_insufficient') ||
        body.contains('insufficient authentication scopes') ||
        body.contains('insufficient') ||
        body.contains('permission') ||
        body.contains('autherror')) {
      return 'Gmail permission was not granted. Reconnect and approve Gmail access on Google’s consent screen.';
    }
    return 'Google did not allow Gmail access (status ${response.statusCode}). Check that Gmail API is enabled and the account is an OAuth test user.';
  }

  Future<String?> _resolveAccountEmail({
    required String accessToken,
    String? idToken,
  }) async {
    final byIdToken = _emailFromIdToken(idToken);
    if (byIdToken != null && byIdToken.isNotEmpty) return byIdToken;

    Object? networkError;
    try {
      final response = await _http
          .get(
            Uri.parse('https://openidconnect.googleapis.com/v1/userinfo'),
            headers: {'Authorization': 'Bearer $accessToken'},
          )
          .timeout(const Duration(seconds: 20));
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body);
        if (json is Map<String, dynamic>) {
          final email = (json['email'] as String?)?.trim();
          if (email != null && email.isNotEmpty) return email;
        }
      }
    } on TimeoutException catch (error) {
      networkError = error;
    } on SocketException catch (error) {
      networkError = error;
    } on http.ClientException catch (error) {
      networkError = error;
    }

    try {
      final response = await _http
          .get(
            Uri.parse('https://gmail.googleapis.com/gmail/v1/users/me/profile'),
            headers: {'Authorization': 'Bearer $accessToken'},
          )
          .timeout(const Duration(seconds: 20));
      if (response.statusCode == 200) {
        final json = jsonDecode(response.body);
        if (json is Map<String, dynamic>) {
          return (json['emailAddress'] as String?)?.trim();
        }
      }
    } on TimeoutException catch (error) {
      networkError = error;
    } on SocketException catch (error) {
      networkError = error;
    } on http.ClientException catch (error) {
      networkError = error;
    }
    if (networkError != null) throw networkError;
    return null;
  }

  String _byokFailureMessage(String stage, Object error) {
    if (error is TimeoutException ||
        error is SocketException ||
        error is http.ClientException) {
      return 'Google approved access, but the app lost its internet connection while $stage. Check the connection and retry BYOK.';
    }
    if (stage == 'saving access securely on this device') {
      return 'Google access was approved, but Vitap Mate could not save it securely. Unlock the device, restart the app, and retry BYOK.';
    }
    if (stage == 'verifying the Google account') {
      return 'Google access was approved, but Vitap Mate could not verify the account email. Retry BYOK and keep the app running.';
    }
    return 'Personal Google setup failed while $stage. Retry BYOK and keep Vitap Mate running.';
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

  String? _validateCollegeAccount({
    required String? email,
    required String expectedUsername,
  }) {
    if (email == null || email.trim().isEmpty) {
      return 'Could not read your Google account email.';
    }
    if (!email.toLowerCase().endsWith('@vitapstudent.ac.in')) {
      return 'Use your @vitapstudent.ac.in email for OTP autofetch.';
    }
    final fromEmail = _usernameFromCollegeEmail(email);
    final fromClient = expectedUsername.trim().toLowerCase().replaceAll(
      RegExp(r'[^a-z0-9]'),
      '',
    );
    final normalizedEmail = fromEmail.replaceAll(RegExp(r'[^a-z0-9]'), '');
    if (fromEmail.isEmpty ||
        fromClient.isEmpty ||
        !normalizedEmail.endsWith(fromClient)) {
      return 'Account mismatch: use the VITAP Gmail address containing your VTOP registration number ($expectedUsername).';
    }
    return null;
  }

  Future<void> _revokeToken(String token) async {
    if (token.trim().isEmpty) return;
    try {
      await _http.post(
        Uri.parse('https://oauth2.googleapis.com/revoke'),
        headers: {'Content-Type': 'application/x-www-form-urlencoded'},
        body: {'token': token},
      );
    } catch (_) {
      // The candidate session is never stored, even if best-effort revocation
      // cannot reach Google.
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
      if (value.contains('noreply.sdc@vitap.ac.in')) return true;
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
