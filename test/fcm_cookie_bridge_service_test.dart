import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'dart:convert';

import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/fcm_cookie_bridge_service.dart';

void main() {
  tearDown(() {
    debugDefaultTargetPlatformOverride = null;
  });

  test('bridge platform support does not require a callback define', () {
    debugDefaultTargetPlatformOverride = TargetPlatform.android;
    expect(fcmCookieBridgePlatformSupported, isTrue);
  });

  test('callback URL comes from the FCM message', () {
    const message = {'callbackUrl': 'https://message.test/cookie/callback'};
    expect(
      resolveCookieCallbackUrlForTest(message),
      'https://message.test/cookie/callback',
    );
    expect(resolveCookieCallbackUrlForTest(const {}), isEmpty);
  });

  test('callback retries temporary failures and then succeeds', () async {
    var calls = 0;
    final delays = <Duration>[];
    final client = MockClient((_) async {
      calls++;
      if (calls == 1) return http.Response('temporary', 500);
      if (calls == 2) return http.Response('slow down', 429);
      return http.Response('{}', 200);
    });

    await postCookieCallbackWithRetry(
      callbackUrl: 'https://bridge.test/cookie/callback',
      requestId: 'request-1',
      responseToken: 'secret',
      cookies: const [
        {'domain': 'vtop.vitap.ac.in', 'name': 'JSESSIONID', 'value': 'abc'},
      ],
      client: client,
      delay: (duration) async => delays.add(duration),
    );

    expect(calls, 3);
    expect(delays, const [Duration(seconds: 1), Duration(seconds: 2)]);
  });

  test('callback does not retry permanent client failures', () async {
    var calls = 0;
    final client = MockClient((_) async {
      calls++;
      return http.Response('bad request', 400);
    });

    await expectLater(
      postCookieCallbackWithRetry(
        callbackUrl: 'https://bridge.test/cookie/callback',
        requestId: 'request-1',
        responseToken: 'secret',
        error: 'login failed',
        client: client,
        delay: (_) async {},
      ),
      throwsA(
        isA<CookieCallbackException>().having(
          (error) => error.retryable,
          'retryable',
          isFalse,
        ),
      ),
    );
    expect(calls, 1);
  });

  test('callback stops after three temporary failures', () async {
    var calls = 0;
    final client = MockClient((_) async {
      calls++;
      return http.Response('unavailable', 503);
    });

    await expectLater(
      postCookieCallbackWithRetry(
        callbackUrl: 'https://bridge.test/cookie/callback',
        requestId: 'request-1',
        responseToken: 'secret',
        cookies: const [
          {'domain': 'vtop.vitap.ac.in', 'name': 'JSESSIONID', 'value': 'abc'},
        ],
        client: client,
        delay: (_) async {},
      ),
      throwsA(isA<StateError>()),
    );
    expect(calls, 3);
  });

  group('bridge credentials payload', () {
    EmailOtpOAuthSession gmail({
      EmailOtpAuthSource source = EmailOtpAuthSource.sharedBuiltIn,
      List<String> scopes = const [
        'https://www.googleapis.com/auth/gmail.modify',
      ],
      String refreshToken = 'refresh-1',
    }) {
      return EmailOtpOAuthSession(
        email: 'me@example.com',
        accessToken: 'access',
        refreshToken: refreshToken,
        scopes: scopes,
        accessTokenExpiryEpochMs: 0,
        authSource: source,
        oauthClientId: source == EmailOtpAuthSource.personalByok
            ? 'byok.apps.googleusercontent.com'
            : null,
        oauthClientSecret: source == EmailOtpAuthSource.personalByok
            ? 'byok-secret'
            : null,
      );
    }

    test('payload is null when not requested', () {
      expect(
        bridgeCredentialsPayload(
          wantCredentials: false,
          serverSignIn: true,
          username: 'yaswanth314',
          password: 'pw',
          gmail: gmail(),
          deleteAfterReading: true,
          sharedClientId: 'shared.apps.googleusercontent.com',
        ),
        isNull,
      );
    });

    test('payload is null without a Gmail session', () {
      expect(
        bridgeCredentialsPayload(
          wantCredentials: true,
          serverSignIn: true,
          username: 'yaswanth314',
          password: 'pw',
          gmail: null,
          deleteAfterReading: true,
          sharedClientId: 'shared.apps.googleusercontent.com',
        ),
        isNull,
      );
    });

    test('payload is null without gmail.modify scope or refresh token', () {
      for (final session in [
        gmail(scopes: const ['openid']),
        gmail(refreshToken: ''),
      ]) {
        expect(
          bridgeCredentialsPayload(
            wantCredentials: true,
            serverSignIn: true,
            username: 'yaswanth314',
            password: 'pw',
            gmail: session,
            deleteAfterReading: true,
            sharedClientId: 'shared.apps.googleusercontent.com',
          ),
          isNull,
        );
      }
    });

    test('payload is null for the shared client when it is not configured', () {
      expect(
        bridgeCredentialsPayload(
          wantCredentials: true,
          serverSignIn: true,
          username: 'yaswanth314',
          password: 'pw',
          gmail: gmail(),
          deleteAfterReading: true,
          sharedClientId: '',
        ),
        isNull,
      );
    });

    test('shared payload uses built-in client id and no secret', () {
      expect(
        bridgeCredentialsPayload(
          wantCredentials: true,
          serverSignIn: true,
          username: 'yaswanth314',
          password: 'pw',
          gmail: gmail(),
          deleteAfterReading: true,
          sharedClientId: 'shared.apps.googleusercontent.com',
        ),
        {
          'username': 'yaswanth314',
          'password': 'pw',
          'gmail': {
            'refresh_token': 'refresh-1',
            'client_id': 'shared.apps.googleusercontent.com',
            'delete_after_reading': true,
          },
        },
      );
    });

    test('byok payload carries client secret', () {
      expect(
        bridgeCredentialsPayload(
          wantCredentials: true,
          serverSignIn: true,
          username: 'yaswanth314',
          password: 'pw',
          gmail: gmail(source: EmailOtpAuthSource.personalByok),
          deleteAfterReading: false,
          sharedClientId: 'shared.apps.googleusercontent.com',
        ),
        {
          'username': 'yaswanth314',
          'password': 'pw',
          'gmail': {
            'refresh_token': 'refresh-1',
            'client_id': 'byok.apps.googleusercontent.com',
            'client_secret': 'byok-secret',
            'delete_after_reading': false,
          },
        },
      );
    });
  });

  test('payload is null without the server sign-in consent', () {
    expect(
      bridgeCredentialsPayload(
        wantCredentials: true,
        serverSignIn: false,
        username: 'yaswanth314',
        password: 'pw',
        gmail: EmailOtpOAuthSession(
          email: 'me@example.com',
          accessToken: 'a',
          refreshToken: 'r',
          scopes: const ['https://www.googleapis.com/auth/gmail.modify'],
          accessTokenExpiryEpochMs: 0,
        ),
        deleteAfterReading: true,
        sharedClientId: 'shared.apps.googleusercontent.com',
      ),
      isNull,
    );
  });

  test('callback body includes username, fcmToken and credentials', () async {
    Map<String, dynamic>? sent;
    final client = MockClient((request) async {
      sent = jsonDecode(request.body) as Map<String, dynamic>;
      return http.Response('{}', 200);
    });

    await postCookieCallbackWithRetry(
      callbackUrl: 'https://bridge.test/cookie/callback',
      requestId: 'request-1',
      responseToken: 'secret',
      cookies: const [
        {'domain': 'vtop.vitap.ac.in', 'name': 'JSESSIONID', 'value': 'abc'},
      ],
      username: '22BCE0001',
      fcmToken: 'fcm-1',
      credentials: const {'password': 'pw'},
      client: client,
    );

    expect(sent!['username'], '22BCE0001');
    expect(sent!['fcmToken'], 'fcm-1');
    expect(sent!['credentials'], {'password': 'pw'});
  });

  test('callback body omits absent optional fields', () async {
    Map<String, dynamic>? sent;
    final client = MockClient((request) async {
      sent = jsonDecode(request.body) as Map<String, dynamic>;
      return http.Response('{}', 200);
    });

    await postCookieCallbackWithRetry(
      callbackUrl: 'https://bridge.test/cookie/callback',
      requestId: 'request-1',
      responseToken: 'secret',
      error: 'No account',
      client: client,
    );

    expect(sent!.containsKey('username'), isFalse);
    expect(sent!.containsKey('fcmToken'), isFalse);
    expect(sent!.containsKey('credentials'), isFalse);
  });
}
