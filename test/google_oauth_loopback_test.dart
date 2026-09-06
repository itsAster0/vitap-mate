import 'dart:async';
import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_appauth/flutter_appauth.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:vitapmate/core/utils/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/core/utils/email_otp/google_oauth_loopback.dart';

void main() {
  test(
    'OTP fetch ignores older mail and never reuses a consumed message',
    () async {
      const session = EmailOtpOAuthSession(
        email: 'student@example.com',
        accessToken: 'access',
        refreshToken: 'refresh',
        scopes: ['https://www.googleapis.com/auth/gmail.modify'],
        accessTokenExpiryEpochMs: 4102444800000,
        authSource: EmailOtpAuthSource.personalByok,
        oauthClientId: 'personal.apps.googleusercontent.com',
      );
      FlutterSecureStorage.setMockInitialValues({
        'email_otp_oauth_session_v1': jsonEncode(session.toJson()),
      });
      final since = DateTime.utc(2026, 9, 20);
      var timestamp = since.subtract(const Duration(milliseconds: 1));
      var modifications = 0;
      final client = MockClient((request) async {
        if (request.method == 'POST') {
          modifications++;
          return http.Response('{}', 200);
        }
        if (request.url.path.endsWith('/messages')) {
          expect(request.url.queryParameters['q'], contains('after:'));
          return http.Response(
            jsonEncode({
              'messages': [
                {'id': 'otp-mail'},
              ],
            }),
            200,
          );
        }
        return http.Response(
          jsonEncode({
            'id': 'otp-mail',
            'internalDate': '${timestamp.millisecondsSinceEpoch}',
            'snippet': 'Your OTP is 123456',
            'payload': {
              'headers': [
                {'name': 'From', 'value': 'noreply.sdc@vitap.ac.in'},
              ],
            },
          }),
          200,
        );
      });
      final service = GoogleEmailOtpAuthService(
        appAuth: const FlutterAppAuth(),
        storage: const FlutterSecureStorage(),
        httpClient: client,
        loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: client),
      );
      expect(await service.fetchLatestOtpSince(sinceUtc: since), isNull);
      expect(modifications, 0);
      timestamp = since.add(const Duration(seconds: 1));
      expect(await service.fetchLatestOtpSince(sinceUtc: since), '123456');
      expect(await service.fetchLatestOtpSince(sinceUtc: since), isNull);
      expect(modifications, 1);
    },
  );

  group('GoogleDesktopOAuthCredentials', () {
    test(
      'parses an installed desktop credential without retaining endpoints',
      () {
        final credentials = GoogleDesktopOAuthCredentials.parseBytes(
          Uint8List.fromList(
            utf8.encode(
              jsonEncode({
                'installed': {
                  'client_id': '123456789.apps.googleusercontent.com',
                  'project_id': 'personal-project',
                  'client_secret': 'secret-value',
                  'auth_uri': 'https://attacker.invalid/authorize',
                  'token_uri': 'https://attacker.invalid/token',
                },
              }),
            ),
          ),
        );

        expect(credentials.clientId, '123456789.apps.googleusercontent.com');
        expect(credentials.projectId, 'personal-project');
        expect(credentials.clientSecret, 'secret-value');
        expect(credentials.redactedClientId, isNot(contains('secret-value')));
      },
    );

    test('rejects web credentials with an actionable error', () {
      expect(
        () => GoogleDesktopOAuthCredentials.parseBytes(
          Uint8List.fromList(
            utf8.encode(
              jsonEncode({
                'web': {'client_id': '123456789.apps.googleusercontent.com'},
              }),
            ),
          ),
        ),
        throwsA(
          isA<GoogleDesktopOAuthException>().having(
            (error) => error.code,
            'code',
            'wrong_client_type',
          ),
        ),
      );
    });

    test('rejects oversized and malformed files', () {
      expect(
        () =>
            GoogleDesktopOAuthCredentials.parseBytes(Uint8List(64 * 1024 + 1)),
        throwsA(
          isA<GoogleDesktopOAuthException>().having(
            (error) => error.code,
            'code',
            'file_too_large',
          ),
        ),
      );
      expect(
        () => GoogleDesktopOAuthCredentials.parseBytes(
          Uint8List.fromList(utf8.encode('{broken')),
        ),
        throwsA(
          isA<GoogleDesktopOAuthException>().having(
            (error) => error.code,
            'code',
            'invalid_json',
          ),
        ),
      );
    });
  });

  group('EmailOtpOAuthSession migration', () {
    test('treats legacy sessions as shared fallback', () {
      final session = EmailOtpOAuthSession.fromJson({
        'email': 'user@vitapstudent.ac.in',
        'accessToken': 'access',
        'refreshToken': 'refresh',
        'scopes': ['https://www.googleapis.com/auth/gmail.modify'],
        'accessTokenExpiryEpochMs': 123,
      });

      expect(session, isNotNull);
      expect(session!.schemaVersion, 1);
      expect(session.authSource, EmailOtpAuthSource.sharedBuiltIn);
      expect(session.authSourceLabel, 'Shared fallback');
    });

    test('round trips personal credentials and source', () {
      const original = EmailOtpOAuthSession(
        email: 'user@vitapstudent.ac.in',
        accessToken: 'access',
        refreshToken: 'refresh',
        scopes: ['https://www.googleapis.com/auth/gmail.modify'],
        accessTokenExpiryEpochMs: 123,
        authSource: EmailOtpAuthSource.personalByok,
        oauthClientId: '123.apps.googleusercontent.com',
        oauthClientSecret: 'secret',
      );

      final restored = EmailOtpOAuthSession.fromJson(original.toJson());
      expect(restored!.authSource, EmailOtpAuthSource.personalByok);
      expect(restored.oauthClientId, original.oauthClientId);
      expect(restored.oauthClientSecret, original.oauthClientSecret);
    });
  });

  group('GoogleEmailOtpAuthService BYOK refresh', () {
    test(
      'polls secure storage until a new personal session is saved',
      () async {
        const previous = EmailOtpOAuthSession(
          email: 'student.24mic7076@vitapstudent.ac.in',
          accessToken: 'old-access',
          refreshToken: 'old-refresh',
          scopes: ['https://www.googleapis.com/auth/gmail.modify'],
          accessTokenExpiryEpochMs: 1,
          authSource: EmailOtpAuthSource.personalByok,
          oauthClientId: 'personal.apps.googleusercontent.com',
        );
        const replacement = EmailOtpOAuthSession(
          email: 'student.24mic7076@vitapstudent.ac.in',
          accessToken: 'new-access',
          refreshToken: 'new-refresh',
          scopes: ['https://www.googleapis.com/auth/gmail.modify'],
          accessTokenExpiryEpochMs: 4102444800000,
          authSource: EmailOtpAuthSource.personalByok,
          oauthClientId: 'personal.apps.googleusercontent.com',
        );
        FlutterSecureStorage.setMockInitialValues({
          'email_otp_oauth_session_v1': jsonEncode(previous.toJson()),
        });
        const storage = FlutterSecureStorage();
        final httpClient = MockClient((_) async => http.Response('{}', 500));
        final service = GoogleEmailOtpAuthService(
          appAuth: const FlutterAppAuth(),
          storage: storage,
          httpClient: httpClient,
          loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: httpClient),
        );

        final detected = service
            .pollForPersonalSession(
              clientId: replacement.oauthClientId!,
              previousSession: previous,
              interval: const Duration(milliseconds: 5),
              timeout: const Duration(milliseconds: 100),
            )
            .first;
        await Future<void>.delayed(const Duration(milliseconds: 12));
        await storage.write(
          key: 'email_otp_oauth_session_v1',
          value: jsonEncode(replacement.toJson()),
        );

        expect((await detected).accessToken, replacement.accessToken);
      },
    );

    test('stops personal session polling at its timeout', () async {
      FlutterSecureStorage.setMockInitialValues({});
      final httpClient = MockClient((_) async => http.Response('{}', 500));
      final service = GoogleEmailOtpAuthService(
        appAuth: const FlutterAppAuth(),
        storage: const FlutterSecureStorage(),
        httpClient: httpClient,
        loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: httpClient),
      );

      final sessions = await service
          .pollForPersonalSession(
            clientId: 'personal.apps.googleusercontent.com',
            interval: const Duration(milliseconds: 5),
            timeout: const Duration(milliseconds: 20),
          )
          .toList();

      expect(sessions, isEmpty);
    });

    test('uses the credentials stored with a personal session', () async {
      const original = EmailOtpOAuthSession(
        email: 'user@vitapstudent.ac.in',
        accessToken: 'expired-access',
        refreshToken: 'personal-refresh',
        scopes: ['https://www.googleapis.com/auth/gmail.modify'],
        accessTokenExpiryEpochMs: 1,
        authSource: EmailOtpAuthSource.personalByok,
        oauthClientId: 'personal.apps.googleusercontent.com',
        oauthClientSecret: 'personal-secret',
      );
      FlutterSecureStorage.setMockInitialValues({
        'email_otp_oauth_session_v1': jsonEncode(original.toJson()),
      });
      late Map<String, String> refreshBody;
      final httpClient = MockClient((request) async {
        refreshBody = request.bodyFields;
        return http.Response(
          jsonEncode({'access_token': 'new-access', 'expires_in': 3600}),
          200,
        );
      });
      final service = GoogleEmailOtpAuthService(
        appAuth: const FlutterAppAuth(),
        storage: const FlutterSecureStorage(),
        httpClient: httpClient,
        loopbackOAuth: GoogleLoopbackOAuthCoordinator(httpClient: httpClient),
      );

      final refreshed = await service.refreshIfNeeded();

      expect(refreshed!.accessToken, 'new-access');
      expect(refreshBody['client_id'], original.oauthClientId);
      expect(refreshBody['client_secret'], original.oauthClientSecret);
      expect(refreshBody['refresh_token'], original.refreshToken);
      expect(refreshed.authSource, EmailOtpAuthSource.personalByok);
    });

    test(
      'does not replace a working session when account validation fails',
      () async {
        const working = EmailOtpOAuthSession(
          email: 'expected@vitapstudent.ac.in',
          accessToken: 'working-access',
          refreshToken: 'working-refresh',
          scopes: ['https://www.googleapis.com/auth/gmail.modify'],
          accessTokenExpiryEpochMs: 4102444800000,
        );
        FlutterSecureStorage.setMockInitialValues({
          'email_otp_oauth_session_v1': jsonEncode(working.toJson()),
        });
        final idTokenPayload = base64Url
            .encode(
              utf8.encode(
                jsonEncode({'email': 'student.23bce0001@vitapstudent.ac.in'}),
              ),
            )
            .replaceAll('=', '');
        final httpClient = MockClient((request) async {
          if (request.url.path == '/token') {
            return http.Response(
              jsonEncode({
                'access_token': 'candidate-access',
                'refresh_token': 'candidate-refresh',
                'expires_in': 3600,
                'scope': 'https://www.googleapis.com/auth/gmail.modify',
                'id_token': 'header.$idTokenPayload.signature',
              }),
              200,
            );
          }
          if (request.url.path == '/revoke') return http.Response('', 200);
          return http.Response('{}', 404);
        });
        final service = GoogleEmailOtpAuthService(
          appAuth: const FlutterAppAuth(),
          storage: const FlutterSecureStorage(),
          httpClient: httpClient,
          loopbackOAuth: GoogleLoopbackOAuthCoordinator(
            httpClient: httpClient,
            timeout: const Duration(seconds: 2),
            random: Random(11),
          ),
        );

        final result = await service.setupByok(
          credentials: const GoogleDesktopOAuthCredentials(
            clientId: 'personal.apps.googleusercontent.com',
            projectId: 'project',
            clientSecret: 'secret',
          ),
          expectedUsername: '24MIC7076',
          openBrowser: (authorizationUri) async {
            final redirect = Uri.parse(
              authorizationUri.queryParameters['redirect_uri']!,
            );
            unawaited(
              http.get(
                redirect.replace(
                  queryParameters: {
                    'code': 'code',
                    'state': authorizationUri.queryParameters['state']!,
                  },
                ),
              ),
            );
            return true;
          },
        );

        expect(result.success, isFalse);
        expect(result.message, contains('Account mismatch'));
        final preserved = await service.loadSession();
        expect(preserved!.accessToken, working.accessToken);
        expect(preserved.refreshToken, working.refreshToken);
        expect(preserved.authSource, EmailOtpAuthSource.sharedBuiltIn);
      },
    );

    test(
      'reports a disabled Gmail API without replacing the session',
      () async {
        const working = EmailOtpOAuthSession(
          email: 'expected@vitapstudent.ac.in',
          accessToken: 'working-access',
          refreshToken: 'working-refresh',
          scopes: ['https://www.googleapis.com/auth/gmail.modify'],
          accessTokenExpiryEpochMs: 4102444800000,
        );
        FlutterSecureStorage.setMockInitialValues({
          'email_otp_oauth_session_v1': jsonEncode(working.toJson()),
        });
        final idTokenPayload = base64Url
            .encode(
              utf8.encode(
                jsonEncode({'email': 'student.24mic7076@vitapstudent.ac.in'}),
              ),
            )
            .replaceAll('=', '');
        final httpClient = MockClient((request) async {
          if (request.url.path == '/token') {
            return http.Response(
              jsonEncode({
                'access_token': 'candidate-access',
                'refresh_token': 'candidate-refresh',
                'expires_in': 3600,
                'scope': 'https://www.googleapis.com/auth/gmail.modify',
                'id_token': 'header.$idTokenPayload.signature',
              }),
              200,
            );
          }
          if (request.url.path == '/gmail/v1/users/me/profile') {
            return http.Response(
              jsonEncode({
                'error': {
                  'status': 'PERMISSION_DENIED',
                  'reason': 'accessNotConfigured',
                },
              }),
              403,
            );
          }
          if (request.url.path == '/revoke') return http.Response('', 200);
          return http.Response('{}', 404);
        });
        final service = GoogleEmailOtpAuthService(
          appAuth: const FlutterAppAuth(),
          storage: const FlutterSecureStorage(),
          httpClient: httpClient,
          loopbackOAuth: GoogleLoopbackOAuthCoordinator(
            httpClient: httpClient,
            timeout: const Duration(seconds: 2),
            random: Random(12),
          ),
        );

        final result = await service.setupByok(
          credentials: const GoogleDesktopOAuthCredentials(
            clientId: 'personal.apps.googleusercontent.com',
            projectId: 'project',
            clientSecret: 'secret',
          ),
          expectedUsername: '24MIC7076',
          openBrowser: (authorizationUri) async {
            final redirect = Uri.parse(
              authorizationUri.queryParameters['redirect_uri']!,
            );
            unawaited(
              http.get(
                redirect.replace(
                  queryParameters: {
                    'code': 'code',
                    'state': authorizationUri.queryParameters['state']!,
                  },
                ),
              ),
            );
            return true;
          },
        );

        expect(result.success, isFalse);
        expect(result.message, contains('Gmail API is not enabled'));
        final preserved = await service.loadSession();
        expect(preserved!.accessToken, working.accessToken);
        expect(preserved.refreshToken, working.refreshToken);
      },
    );
  });

  group('GoogleLoopbackOAuthCoordinator', () {
    const credentials = GoogleDesktopOAuthCredentials(
      clientId: '123.apps.googleusercontent.com',
      projectId: 'project',
      clientSecret: 'secret',
    );

    test('handles a valid state callback and exchanges with PKCE', () async {
      late Map<String, String> tokenBody;
      final progress = <String>[];
      var appReturnedToForeground = false;
      final tokenClient = MockClient((request) async {
        expect(appReturnedToForeground, isTrue);
        tokenBody = request.bodyFields;
        return http.Response(
          jsonEncode({
            'access_token': 'access-token',
            'refresh_token': 'refresh-token',
            'expires_in': 3600,
            'scope': 'openid https://www.googleapis.com/auth/gmail.modify',
          }),
          200,
        );
      });
      final coordinator = GoogleLoopbackOAuthCoordinator(
        httpClient: tokenClient,
        timeout: const Duration(seconds: 2),
        random: Random(4),
      );

      final result = await coordinator.authorize(
        credentials: credentials,
        scopes: const [
          'openid',
          'https://www.googleapis.com/auth/gmail.modify',
        ],
        openBrowser: (authorizationUri) async {
          final redirect = Uri.parse(
            authorizationUri.queryParameters['redirect_uri']!,
          );
          final callback = redirect.replace(
            queryParameters: {
              'code': 'authorization-code',
              'state': authorizationUri.queryParameters['state']!,
            },
          );
          unawaited(http.get(callback));
          return true;
        },
        beforeTokenExchange: () async {
          appReturnedToForeground = true;
        },
        onProgress: progress.add,
      );

      expect(result.accessToken, 'access-token');
      expect(result.refreshToken, 'refresh-token');
      expect(tokenBody['code'], 'authorization-code');
      expect(tokenBody['code_verifier'], isNotEmpty);
      expect(tokenBody['redirect_uri'], startsWith('http://127.0.0.1:'));
      expect(tokenBody['client_secret'], 'secret');
      expect(progress, contains(contains('Exchanging the Google code')));
      expect(progress, contains(contains('Return to Vitap Mate')));
      expect(coordinator.isActive, isFalse);
    });

    test('retries a transient token exchange network failure', () async {
      var attempts = 0;
      final tokenClient = MockClient((request) async {
        attempts++;
        if (attempts == 1) {
          throw http.ClientException('temporary connection loss', request.url);
        }
        return http.Response(
          jsonEncode({
            'access_token': 'access-token',
            'refresh_token': 'refresh-token',
            'expires_in': 3600,
            'scope': 'openid https://www.googleapis.com/auth/gmail.modify',
          }),
          200,
        );
      });
      final coordinator = GoogleLoopbackOAuthCoordinator(
        httpClient: tokenClient,
        timeout: const Duration(seconds: 2),
        random: Random(5),
      );

      final result = await coordinator.authorize(
        credentials: credentials,
        scopes: const [
          'openid',
          'https://www.googleapis.com/auth/gmail.modify',
        ],
        openBrowser: (authorizationUri) async {
          final redirect = Uri.parse(
            authorizationUri.queryParameters['redirect_uri']!,
          );
          unawaited(
            http.get(
              redirect.replace(
                queryParameters: {
                  'code': 'authorization-code',
                  'state': authorizationUri.queryParameters['state']!,
                },
              ),
            ),
          );
          return true;
        },
      );

      expect(result.accessToken, 'access-token');
      expect(attempts, 2);
      expect(coordinator.isActive, isFalse);
    });

    test('rejects a token that omitted Gmail permission', () async {
      final coordinator = GoogleLoopbackOAuthCoordinator(
        httpClient: MockClient(
          (_) async => http.Response(
            jsonEncode({
              'access_token': 'access-token',
              'refresh_token': 'refresh-token',
              'expires_in': 3600,
              'scope': 'openid email profile',
            }),
            200,
          ),
        ),
        timeout: const Duration(seconds: 2),
        random: Random(6),
      );

      final future = coordinator.authorize(
        credentials: credentials,
        scopes: const [
          'openid',
          'https://www.googleapis.com/auth/gmail.modify',
        ],
        openBrowser: (authorizationUri) async {
          final redirect = Uri.parse(
            authorizationUri.queryParameters['redirect_uri']!,
          );
          unawaited(
            http.get(
              redirect.replace(
                queryParameters: {
                  'code': 'authorization-code',
                  'state': authorizationUri.queryParameters['state']!,
                },
              ),
            ),
          );
          return true;
        },
      );

      await expectLater(
        future,
        throwsA(
          isA<GoogleDesktopOAuthException>().having(
            (error) => error.code,
            'code',
            'missing_gmail_scope',
          ),
        ),
      );
      expect(coordinator.isActive, isFalse);
    });

    test('rejects a mismatched state and closes the listener', () async {
      final coordinator = GoogleLoopbackOAuthCoordinator(
        httpClient: MockClient((_) async => http.Response('{}', 500)),
        timeout: const Duration(seconds: 2),
        random: Random(7),
      );

      final future = coordinator.authorize(
        credentials: credentials,
        scopes: const ['openid'],
        openBrowser: (authorizationUri) async {
          final redirect = Uri.parse(
            authorizationUri.queryParameters['redirect_uri']!,
          );
          unawaited(
            http.get(
              redirect.replace(
                queryParameters: {'code': 'code', 'state': 'wrong-state'},
              ),
            ),
          );
          return true;
        },
      );

      await expectLater(
        future,
        throwsA(
          isA<GoogleDesktopOAuthException>().having(
            (error) => error.code,
            'code',
            'state_mismatch',
          ),
        ),
      );
      expect(coordinator.isActive, isFalse);
    });

    test('times out and cleans up when no callback arrives', () async {
      final coordinator = GoogleLoopbackOAuthCoordinator(
        httpClient: MockClient((_) async => http.Response('{}', 500)),
        timeout: const Duration(milliseconds: 25),
        random: Random(8),
      );

      await expectLater(
        coordinator.authorize(
          credentials: credentials,
          scopes: const ['openid'],
          openBrowser: (_) async => true,
        ),
        throwsA(
          isA<GoogleDesktopOAuthException>().having(
            (error) => error.code,
            'code',
            'timeout',
          ),
        ),
      );
      expect(coordinator.isActive, isFalse);
    });
  });
}
