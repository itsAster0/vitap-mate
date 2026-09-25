import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:vitapmate/core/vtop_backend/vtop_authenticator.dart';
import 'package:vitapmate/core/vtop_backend/vtop_server_settings.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:vitapmate/src/api/vtop/vtop_client.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

/// Stands in for the opaque Rust client; the authenticator only passes it
/// to the injected resume/export hooks.
class _FakeClient implements VtopClient {
  SessionState? session;

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

const _settings = VtopServerSettings(
  url: 'https://vtop.example.com',
  apiKey: '',
);

RemoteVtopAuthenticator _authenticator(
  Future<http.Response> Function(http.Request) handler, {
  GmailLoginAccess? gmail,
  Future<void> Function()? onGmailTokenRejected,
}) => RemoteVtopAuthenticator(
  settings: _settings,
  credentials: () async => (username: '22bce0000', password: 'secret'),
  gmail: () async => gmail,
  onGmailTokenRejected: onGmailTokenRejected,
  httpClient: MockClient(handler),
  resume: (client, session) => (client as _FakeClient).session = session,
  export: (client) => (client as _FakeClient).session!,
);

void main() {
  test('login loads the returned session, login time included', () async {
    late Map<String, dynamic> sent;
    final auth = _authenticator((request) async {
      expect(request.url.path, '/v1/auth/login');
      sent = jsonDecode(request.body) as Map<String, dynamic>;
      return http.Response(
        jsonEncode({
          'status': 'authenticated',
          'session': {
            'cookies': 'JSESSIONID=abc',
            'csrf_token': 'csrf',
            'registration_number': '22BCE0000',
            'logged_in_at': 1700000000,
          },
        }),
        200,
      );
    });
    final client = _FakeClient();

    await auth.login(client);

    expect(sent, {'username': '22BCE0000', 'password': 'secret'});
    expect(client.session!.csrfToken, 'csrf');
    expect(client.session!.loggedInAt, BigInt.from(1700000000));
  });

  test(
    'an OTP challenge keeps the pending session and asks for the code',
    () async {
      final requests = <String, Map<String, dynamic>>{};
      final auth = _authenticator((request) async {
        requests[request.url.path] =
            jsonDecode(request.body) as Map<String, dynamic>;
        if (request.url.path == '/v1/auth/login') {
          return http.Response(
            jsonEncode({
              'status': 'otp_required',
              'message': 'OTP sent',
              'issued_at': 1700000100,
              'session': {
                'cookies': 'JSESSIONID=pending',
                'csrf_token': 'csrf-otp',
                'otp_issued_at': 1700000100,
              },
            }),
            200,
          );
        }
        return http.Response(
          jsonEncode({
            'status': 'authenticated',
            'session': {
              'cookies': 'JSESSIONID=done',
              'csrf_token': 'csrf-2',
              'registration_number': '22BCE0000',
              'logged_in_at': 1700000200,
            },
          }),
          200,
        );
      });
      final client = _FakeClient();

      await expectLater(
        auth.login(client),
        throwsA(VtopError.otpRequired('OTP sent', BigInt.from(1700000100))),
      );
      expect(client.session!.otpIssuedAt, BigInt.from(1700000100));

      await auth.submitOtp(client, '123456');

      expect(requests['/v1/auth/otp'], {
        'session': {
          'cookies': 'JSESSIONID=pending',
          'csrf_token': 'csrf-otp',
          'otp_issued_at': 1700000100,
        },
        'otp': '123456',
      });
      expect(client.session!.cookies, 'JSESSIONID=done');
    },
  );

  test('server errors come back as VtopError', () async {
    final auth = _authenticator(
      (_) async => http.Response(
        jsonEncode({
          'error': {'code': 'invalid_credentials', 'message': 'bad'},
        }),
        401,
      ),
    );
    await expectLater(
      auth.login(_FakeClient()),
      throwsA(const VtopError.invalidCredentials()),
    );
  });

  test('a Gmail token rides along and a rejected one gets refreshed', () async {
    late Map<String, dynamic> sent;
    var refreshed = 0;
    final auth = _authenticator(
      (request) async {
        sent = jsonDecode(request.body) as Map<String, dynamic>;
        return http.Response(
          jsonEncode({
            'status': 'otp_required',
            'message': 'OTP sent',
            'issued_at': 1,
            'gmail': 'gmail_unauthorized',
            'session': {'cookies': 'JSESSIONID=p', 'otp_issued_at': 1},
          }),
          200,
        );
      },
      gmail: (
        accessToken: 'gmail-token',
        expiresAtUnix: 1790000000,
        deleteAfterReading: true,
      ),
      onGmailTokenRejected: () async => refreshed++,
    );

    await expectLater(
      auth.login(_FakeClient()),
      throwsA(isA<VtopError_OTPRequired>()),
    );

    expect(sent['gmail'], {
      'access_token': 'gmail-token',
      'expires_at': 1790000000,
      'delete_after_reading': true,
    });
    expect(refreshed, 1);
  });
}
