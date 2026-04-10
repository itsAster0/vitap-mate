import 'dart:convert';

import 'package:flutter_appauth/flutter_appauth.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/core/services/app_logger.dart';
import 'package:vitapmate/core/services/storage.dart';

const googleOauthClientId =
    '95428715364-4f11adlt9kjaubvc3jif86cectghm448.apps.googleusercontent.com';
const googleOauthRedirectScheme =
    'com.googleusercontent.apps.95428715364-4f11adlt9kjaubvc3jif86cectghm448';
const googleOauthRedirectUrl = '$googleOauthRedirectScheme:/oauthredirect';

const googleEmailScopes = <String>[
  'email',
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

class GmailOAuthService {
  GmailOAuthService(
    this._logger,
    this._secureAccountStore, [
    FlutterAppAuth? appAuth,
  ]) : _appAuth = appAuth ?? const FlutterAppAuth();

  final AppLogger _logger;
  final SecureAccountStore _secureAccountStore;
  final FlutterAppAuth _appAuth;

  Future<String> authorizeForGmail({String? loginHint}) async {
    try {
      final response = await _appAuth.authorizeAndExchangeCode(
        AuthorizationTokenRequest(
          googleOauthClientId,
          googleOauthRedirectUrl,
          serviceConfiguration: _googleServiceConfiguration,
          scopes: gmailOauthScopes,
          loginHint: loginHint,
          promptValues: const [Prompt.consent],
          additionalParameters: const {'access_type': 'offline'},
        ),
      );
      final email = _emailFromTokenResponse(response).trim().toLowerCase();
      if (!email.endsWith('@vitapstudent.ac.in')) {
        throw const FormatException(
          'Use your vitapstudent.ac.in Google account to continue.',
        );
      }
      await _storeTokenResponse(email: email, response: response);
      _logger.info('auth', 'College email: $email');
      return email;
    } catch (error, stackTrace) {
      _logger.error(
        'gmail',
        'Gmail OAuth failed',
        error: error,
        stackTrace: stackTrace,
      );
      if (error is GoogleReauthRequiredException) {
        rethrow;
      }
      if (error is FormatException) {
        rethrow;
      }
      throw GoogleReauthRequiredException(
        'Gmail access is required to read OTP emails. Please allow Gmail access to continue.',
        cause: error,
      );
    }
  }

  Future<void> refreshAccessToken() async {
    final stored = await _secureAccountStore.readGoogleTokens();
    final refreshToken = stored?['refreshToken'] as String?;
    final email = stored?['email'] as String?;
    if (refreshToken == null || refreshToken.isEmpty) {
      throw const GoogleReauthRequiredException(
        'No Gmail refresh token is saved. Please reconnect Gmail.',
      );
    }
    final response = await _appAuth.token(
      TokenRequest(
        googleOauthClientId,
        googleOauthRedirectUrl,
        serviceConfiguration: _googleServiceConfiguration,
        scopes: gmailOauthScopes,
        refreshToken: refreshToken,
      ),
    );
    await _storeTokenResponse(
      email: email ?? '',
      response: response,
      fallbackRefreshToken: refreshToken,
    );
  }

  Future<void> _storeTokenResponse({
    required String email,
    required TokenResponse response,
    String? fallbackRefreshToken,
  }) async {
    final refreshToken = response.refreshToken ?? fallbackRefreshToken;
    if (refreshToken == null || refreshToken.isEmpty) {
      throw const GoogleReauthRequiredException(
        'Google did not return a Gmail refresh token. Remove this app from your Google account access, then reconnect Gmail and allow consent again.',
      );
    }
    await _secureAccountStore.writeGoogleTokens(
      email: email,
      accessToken: response.accessToken,
      refreshToken: refreshToken,
      idToken: response.idToken,
      scopes: response.scopes ?? gmailOauthScopes,
      accessTokenExpiresAt: response.accessTokenExpirationDateTime,
      tokenType: response.tokenType,
    );
    _logger.info(
      'gmail',
      'Saved Gmail access ${_maskedToken(response.accessToken)} and refresh ${_maskedToken(refreshToken)}',
    );
  }

  String _maskedToken(String? token) {
    if (token == null || token.isEmpty) return '<missing>';
    if (token.length <= 12) return '${token.length} chars';
    return '${token.substring(0, 6)}...${token.substring(token.length - 4)} '
        '(${token.length} chars)';
  }

  String _emailFromTokenResponse(TokenResponse response) {
    final idTokenEmail = _emailFromIdToken(response.idToken);
    if (idTokenEmail != null && idTokenEmail.isNotEmpty) {
      return idTokenEmail;
    }
    final tokenEmail = response.tokenAdditionalParameters?['email']?.toString();
    if (tokenEmail != null && tokenEmail.isNotEmpty) {
      return tokenEmail;
    }
    throw const GoogleReauthRequiredException(
      'Google did not return an email address. Please reconnect Gmail and allow email access.',
    );
  }

  String? _emailFromIdToken(String? idToken) {
    if (idToken == null || idToken.isEmpty) return null;
    final parts = idToken.split('.');
    if (parts.length < 2) return null;
    final normalizedPayload = base64Url.normalize(parts[1]);
    final payload = utf8.decode(base64Url.decode(normalizedPayload));
    final claims = jsonDecode(payload) as Map<String, dynamic>;
    return claims['email']?.toString();
  }
}
