import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter_appauth/flutter_appauth.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/src/api/email_otp.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';
import 'package:vitapmate/vtop/email_otp/email_otp_oauth_session.dart';
import 'package:vitapmate/vtop/email_otp/gmail_message.dart';
import 'package:vitapmate/vtop/email_otp/google_oauth_loopback.dart';
import 'package:vitapmate/vtop/email_otp/mail_forwarding_service.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';

export 'package:vitapmate/vtop/email_otp/email_otp_oauth_session.dart';

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
  _gmailModifyScope,
];
const _googleServiceConfiguration = AuthorizationServiceConfiguration(
  authorizationEndpoint: 'https://accounts.google.com/o/oauth2/v2/auth',
  tokenEndpoint: 'https://oauth2.googleapis.com/token',
);
const _oauthStorageKey = 'email_otp_oauth_session_v1';
const _gmailModifyScope = 'https://www.googleapis.com/auth/gmail.modify';
const _gmailRevoked =
    'Google authorization expired or was revoked. Reconnect Gmail.';

typedef GmailOtpFinder =
    Future<GmailOtpCode?> Function({
      required String accessToken,
      BigInt? expiresAtUnix,
      required BigInt issuedAtUnix,
      required List<String> skipMessageIds,
      required String? flow,
    });

typedef GmailOtpTidier =
    Future<void> Function({
      required String accessToken,
      required String messageId,
      required bool deleteAfterReading,
      required String? flow,
    });

@Riverpod(keepAlive: true)
GoogleEmailOtpAuthService googleEmailOtpAuthService(Ref ref) {
  // The app's client, so these requests are logged like the rest.
  final httpClient = sharedAppHttpClient;
  return GoogleEmailOtpAuthService(
    appAuth: const FlutterAppAuth(),
    storage: const FlutterSecureStorage(),
    httpClient: httpClient,
    loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: httpClient),
  );
}

/// Gmail access is connected (own key or shared). Server sign-in needs it.
@Riverpod(keepAlive: true)
Future<bool> emailOtpReady(Ref ref) =>
    ref.watch(googleEmailOtpAuthServiceProvider).isReady();

/// Some way of reading the OTP is set up: Gmail or a forwarding address.
@Riverpod(keepAlive: true)
Future<bool> anyEmailOtpReady(Ref ref) async =>
    await ref.watch(emailOtpReadyProvider.future) ||
    (!ref.watch(onDeviceModeProvider) &&
        await ref.watch(mailForwardingServiceProvider).isReady());

@Riverpod(keepAlive: true)
Future<bool> emailOtpSetupNeeded(Ref ref) async =>
    !await ref.watch(anyEmailOtpReadyProvider.future);

class GoogleEmailOtpAuthService {
  GoogleEmailOtpAuthService({
    required FlutterAppAuth appAuth,
    required FlutterSecureStorage storage,
    required http.Client httpClient,
    required GoogleLoopbackOAuthCoordinator loopbackOAuth,
    GmailOtpFinder? findOtp,
    GmailOtpTidier? tidyUp,
  }) : _findOtp = findOtp ?? gmailFindOtp,
       _tidyUp = tidyUp ?? gmailTidyUp,
       _appAuth = appAuth,
       _storage = storage,
       _http = httpClient,
       _loopbackOAuth = loopbackOAuth;

  final FlutterAppAuth _appAuth;
  final FlutterSecureStorage _storage;
  final http.Client _http;
  final GoogleLoopbackOAuthCoordinator _loopbackOAuth;
  final Set<String> _consumedOtpMessageIds = {};

  /// Rust's Gmail OTP reader (`vtop_core::gmail`) by default; swappable so
  /// tests run without the native library.
  final GmailOtpFinder _findOtp;
  final GmailOtpTidier _tidyUp;

  Future<EmailOtpOAuthSession?> loadSession() async {
    final raw = await _storage.read(key: _oauthStorageKey);
    if (raw == null || raw.isEmpty) return null;
    try {
      return switch (jsonDecode(raw)) {
        final Map<String, dynamic> json => EmailOtpOAuthSession.fromJson(json),
        _ => null,
      };
    } catch (_) {
      return null;
    }
  }

  Future<void> _saveSession(EmailOtpOAuthSession session) => _storage.write(
    key: _oauthStorageKey,
    value: jsonEncode(session.toJson()),
  );

  Future<void> clearSession() async {
    await _loopbackOAuth.cancel();
    await _storage.delete(key: _oauthStorageKey);
  }

  Future<void> cancelByokSetup() => _loopbackOAuth.cancel();

  GoogleDesktopOAuthCredentials parseByokCredentials(List<int> bytes) =>
      GoogleDesktopOAuthCredentials.parseBytes(
        bytes is Uint8List ? bytes : Uint8List.fromList(bytes),
      );

  Future<bool> isReady() async {
    final session = await loadSession();
    if (session == null) return false;
    if (session.authSource == EmailOtpAuthSource.sharedBuiltIn &&
        !sharedGoogleOAuthEnabled) {
      return false;
    }
    return session.hasGmailScope && session.refreshToken.isNotEmpty;
  }

  /// Gmail access for the server ([gmailServerGrant]), or null when Gmail
  /// is not connected.
  Future<Map<String, dynamic>?> serverGrant({
    required bool deleteAfterReading,
  }) async {
    if (!await isReady()) return null;
    final session = await loadSession();
    if (session == null) return null;
    return gmailServerGrant(
      session,
      sharedClientId: googleOauthClientId,
      deleteAfterReading: deleteAfterReading,
    );
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
        return EmailOtpSetupResult.failed(validationMessage, email: email);
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
      await _saveSession(session);
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
      return EmailOtpSetupResult.ok(
        'Personal Gmail access is connected.',
        email: email,
      );
    } on GoogleDesktopOAuthException catch (error) {
      return EmailOtpSetupResult.failed(error.message);
    } catch (error, stackTrace) {
      Log.error(
        'email_otp.oauth',
        'Personal OAuth setup failed while $stage: $error\n$stackTrace',
      );
      return EmailOtpSetupResult.failed(_byokFailureMessage(stage, error));
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
      return const EmailOtpSetupResult.failed(
        'Shared Google access is not configured in this build.',
      );
    }
    try {
      Log.info(
        'email_otp.oauth',
        'Starting identity OAuth request with scopes: ${googleEmailScopes.join(', ')}',
      );
      final identityTokens = await loggedNetCall(
        'Google sign-in (identity)',
        () => _appAuth.authorizeAndExchangeCode(
          AuthorizationTokenRequest(
            googleOauthClientId,
            googleOauthRedirectUrl,
            serviceConfiguration: _googleServiceConfiguration,
            scopes: googleEmailScopes,
            promptValues: const ['consent'],
            additionalParameters: const {'access_type': 'offline'},
          ),
        ),
      );
      if (identityTokens.accessToken == null) {
        return const EmailOtpSetupResult.failed(
          'Google sign-in was cancelled.',
        );
      }

      final email = await _resolveAccountEmail(
        accessToken: identityTokens.accessToken!,
        idToken: identityTokens.idToken,
      );
      if (email == null || email.isEmpty) {
        return const EmailOtpSetupResult.failed(
          'Could not read your Google account email.',
        );
      }
      final validationMessage = _validateCollegeAccount(
        email: email,
        expectedUsername: expectedUsername,
      );
      if (validationMessage != null) {
        return EmailOtpSetupResult.failed(validationMessage, email: email);
      }
      return EmailOtpSetupResult.ok(
        'Email verified. Continue to token step.',
        email: email,
      );
    } catch (error, stackTrace) {
      Log.error('email_otp.oauth', 'Identity step failed: $error\n$stackTrace');
      return EmailOtpSetupResult.failed(_friendlyOauthError(error));
    }
  }

  Future<EmailOtpSetupResult> setupGmailTokenStep({
    required String email,
  }) async {
    if (googleOauthClientId.isEmpty) {
      return const EmailOtpSetupResult.failed(
        'Shared Google access is not configured in this build.',
      );
    }
    try {
      Log.info(
        'email_otp.oauth',
        'Starting shared Gmail token OAuth request with scopes: ${gmailOauthScopes.join(', ')}',
      );
      final gmailTokens = await loggedNetCall(
        'Google sign-in (Gmail access)',
        () => _appAuth.authorizeAndExchangeCode(
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
        ),
      );
      if (gmailTokens.accessToken == null) {
        return const EmailOtpSetupResult.failed(
          'Gmail permission setup was cancelled.',
        );
      }
      final grantedScopes = gmailTokens.scopes?.isNotEmpty == true
          ? gmailTokens.scopes!
          : gmailOauthScopes;
      if (!grantedScopes.contains(_gmailModifyScope)) {
        return const EmailOtpSetupResult.failed(
          'Google sign-in completed, but Gmail access was not granted. Retry and approve the Gmail permission.',
        );
      }
      try {
        await _verifyGmailAccess(gmailTokens.accessToken!);
      } on GoogleDesktopOAuthException catch (error) {
        return EmailOtpSetupResult.failed(error.message);
      }
      final refreshToken = (gmailTokens.refreshToken ?? '').trim();
      if (refreshToken.isEmpty) {
        return const EmailOtpSetupResult.failed(
          'Could not get a refresh token for Gmail access.',
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
      await _saveSession(session);
      return EmailOtpSetupResult.ok(
        'Email OTP autofetch is enabled.',
        email: email,
      );
    } catch (error, stackTrace) {
      Log.error('email_otp.oauth', 'Token step failed: $error\n$stackTrace');
      return EmailOtpSetupResult.failed(_friendlyOauthError(error));
    }
  }

  /// The session with an access token valid for at least [minValidity],
  /// refreshing it when needed (or always, with [force], e.g. after Gmail
  /// rejected the current token).
  Future<EmailOtpOAuthSession?> refreshIfNeeded({
    bool force = false,
    Duration minValidity = const Duration(seconds: 30),
  }) async {
    final session = await loadSession();
    if (session == null) return null;
    if (!force && !session.expiresWithin(minValidity)) return session;
    if (session.authSource == EmailOtpAuthSource.personalByok) {
      return _refreshByokSession(session);
    }
    if (!sharedGoogleOAuthEnabled) {
      await clearSession();
      return null;
    }
    final token = await loggedNetCall(
      'Google token refresh',
      () => _appAuth.token(
        TokenRequest(
          googleOauthClientId,
          googleOauthRedirectUrl,
          serviceConfiguration: _googleServiceConfiguration,
          refreshToken: session.refreshToken,
          scopes: gmailOauthScopes,
        ),
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
    await _saveSession(refreshed);
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
    final secret = session.oauthClientSecret?.trim();
    final response = await _http.post(
      Uri.parse('https://oauth2.googleapis.com/token'),
      body: {
        'client_id': clientId,
        'refresh_token': session.refreshToken,
        'grant_type': 'refresh_token',
        if (secret != null && secret.isNotEmpty) 'client_secret': secret,
      },
    );
    final decoded = decodeJsonObject(response.body);
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
    await _saveSession(refreshed);
    return refreshed;
  }

  /// VTOP's OTP from Gmail sent after [sinceUtc], or null when it has not
  /// arrived yet. Reading and parsing run in Rust (`vtop_core::gmail`), the
  /// same code the server engine uses; this side only supplies a fresh token.
  ///
  /// A rejected token is refreshed once and retried before Gmail is
  /// disconnected, since a 401 usually just means the token expired.
  Future<String?> fetchLatestOtpSince({
    required DateTime sinceUtc,
    bool deleteAfterReading = true,
  }) async {
    var session = await refreshIfNeeded();
    if (session == null) return null;

    Future<GmailOtpCode?> find(EmailOtpOAuthSession session) => _findOtp(
      accessToken: session.accessToken,
      expiresAtUnix: BigInt.from(session.accessTokenExpiryEpochMs ~/ 1000),
      issuedAtUnix: BigInt.from(sinceUtc.millisecondsSinceEpoch ~/ 1000),
      skipMessageIds: _consumedOtpMessageIds.toList(),
      flow: currentFlow,
    );

    GmailOtpCode? found;
    try {
      found = await find(session);
    } on VtopError catch (error) {
      if (_gmailErrorCode(error) != 'gmail_unauthorized') {
        throw _gmailStateError(error);
      }
      session = await refreshIfNeeded(force: true);
      if (session == null) return null;
      try {
        found = await find(session);
      } on VtopError catch (retryError) {
        if (_gmailErrorCode(retryError) == 'gmail_unauthorized') {
          await clearSession();
          throw StateError(_gmailRevoked);
        }
        throw _gmailStateError(retryError);
      }
    }
    if (found == null) return null;

    _consumedOtpMessageIds.add(found.messageId);
    await _tidyUp(
      accessToken: session.accessToken,
      messageId: found.messageId,
      deleteAfterReading: deleteAfterReading,
      flow: currentFlow,
    );
    return found.code;
  }

  String? _gmailErrorCode(VtopError error) =>
      error.maybeWhen(configurationError: (code) => code, orElse: () => null);

  StateError _gmailStateError(
    VtopError error,
  ) => StateError(switch (_gmailErrorCode(error)) {
    'gmail_forbidden' =>
      'Gmail refused access. Check the Gmail API is enabled and access was granted, or reconnect Gmail.',
    'gmail_unavailable' => 'Could not reach Gmail. Check your connection.',
    _ => 'Unable to read Gmail inbox ($error).',
  });

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
    if (latest == null || !isFromVtopOtpSender(latest)) return null;

    final internalDateMs = int.tryParse('${latest['internalDate']}') ?? 0;
    final receivedAt = DateTime.fromMillisecondsSinceEpoch(
      internalDateMs,
      isUtc: true,
    );
    final text = gmailMessageText(latest);
    final collapsed = text.replaceAll(RegExp(r'\s+'), ' ').trim();
    final snippet = collapsed.length > 500
        ? '${collapsed.substring(0, 500)}...'
        : collapsed;
    final otp = RegExp(r'(?<!\d)(\d{6})(?!\d)').firstMatch(text)?.group(1);
    final id = latest['id'] as String?;
    if (otp != null && id != null && id.isNotEmpty) {
      await _tidyUp(
        accessToken: session.accessToken,
        messageId: id,
        deleteAfterReading: deleteAfterReading,
        flow: currentFlow,
      );
    }

    return LatestInfoEmail(
      receivedAt: receivedAt,
      subject: gmailHeader(latest, 'subject') ?? '(no subject)',
      snippet: snippet.isEmpty ? '(empty message)' : snippet,
      otp: otp,
    );
  }

  Future<Map<String, dynamic>?> _fetchFirstListedMessage(
    String listResponseBody,
    String accessToken,
  ) async {
    final raw = jsonDecode(listResponseBody) as Map<String, dynamic>;
    final first = switch (raw['messages']) {
      [final Map<String, dynamic> first, ...] => first,
      _ => null,
    };
    final id = first?['id'] as String?;
    if (id == null || id.isEmpty) return null;

    final res = await _http.get(
      Uri.parse(
        'https://gmail.googleapis.com/gmail/v1/users/me/messages/$id?format=full',
      ),
      headers: {'Authorization': 'Bearer $accessToken'},
    );
    if (res.statusCode != 200) return null;
    return switch (jsonDecode(res.body)) {
      final Map<String, dynamic> message => message,
      _ => null,
    };
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
      throw StateError(_gmailRevoked);
    }
    if (response.statusCode == 403) {
      final message = _gmailAccessFailureMessage(response);
      // A disabled API can be enabled without reconnecting.
      if (!_gmailApiDisabled(response.body.toLowerCase())) {
        await clearSession();
      }
      throw StateError(message);
    }
  }

  static bool _gmailApiDisabled(String body) =>
      body.contains('accessnotconfigured') ||
      body.contains('service_disabled') ||
      body.contains('api has not been used');

  String _gmailAccessFailureMessage(http.Response response) {
    final body = response.body.toLowerCase();
    if (response.statusCode == 401) {
      return _gmailRevoked;
    }
    if (_gmailApiDisabled(body)) {
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
    // Covers insufficientPermissions, ACCESS_TOKEN_SCOPE_INSUFFICIENT and
    // "insufficient authentication scopes".
    if (body.contains('insufficient') ||
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

    // The OpenID userinfo first, then the Gmail profile; a network error is
    // only thrown once both have been tried.
    Object? networkError;
    for (final (url, field) in const [
      ('https://openidconnect.googleapis.com/v1/userinfo', 'email'),
      (
        'https://gmail.googleapis.com/gmail/v1/users/me/profile',
        'emailAddress',
      ),
    ]) {
      try {
        final response = await _http
            .get(
              Uri.parse(url),
              headers: {'Authorization': 'Bearer $accessToken'},
            )
            .timeout(const Duration(seconds: 20));
        if (response.statusCode != 200) continue;
        final json = jsonDecode(response.body);
        if (json is! Map<String, dynamic>) continue;
        final email = (json[field] as String?)?.trim();
        if (email != null && email.isNotEmpty) return email;
      } catch (error) {
        if (!isNetworkError(error)) rethrow;
        networkError = error;
      }
    }
    if (networkError != null) throw networkError;
    return null;
  }

  String _byokFailureMessage(String stage, Object error) {
    if (isNetworkError(error)) {
      return 'Google approved access, but the app lost its internet connection while $stage. Check the connection and retry BYOK.';
    }
    return switch (stage) {
      'saving access securely on this device' =>
        'Google access was approved, but Vitap Mate could not save it securely. Unlock the device, restart the app, and retry BYOK.',
      'verifying the Google account' =>
        'Google access was approved, but Vitap Mate could not verify the account email. Retry BYOK and keep the app running.',
      _ =>
        'Personal Google setup failed while $stage. Retry BYOK and keep Vitap Mate running.',
    };
  }

  String? _emailFromIdToken(String? idToken) {
    if (idToken?.split('.') case [_, final payload, ...]) {
      final normalized = base64Url.normalize(payload);
      try {
        final json = jsonDecode(utf8.decode(base64Url.decode(normalized)));
        if (json is! Map<String, dynamic>) return null;
        return (json['email'] as String?)?.trim();
      } catch (_) {
        return null;
      }
    }
    return null;
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
    // The registration number is the last dot-separated part before the @.
    final fromEmail =
        email
            .split('@')
            .first
            .split('.')
            .where((part) => part.isNotEmpty)
            .lastOrNull
            ?.toLowerCase() ??
        '';
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

  String _friendlyOauthError(Object error) {
    final text = error.toString().toLowerCase();
    if (text.contains('cancelled')) {
      return 'Google sign-in was cancelled.';
    }
    if (text.contains('null_intent') || text.contains('failed to authorize')) {
      return 'Could not launch Google authorization. Please try again.';
    }
    return 'OAuth setup failed. Please try again.';
  }
}
