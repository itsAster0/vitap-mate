import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:vitapmate/vtop/http/vtop_platform_http.dart';
import 'package:vitapmate/src/api/platform_http.dart';

PlatformHttpRequest _request({int timeoutMs = 5000}) => PlatformHttpRequest(
  method: 'POST',
  url: 'https://proxy.example/vtop/login',
  headers: const [('user-agent', 'Firefox'), ('cookie', 'JSESSIONID=a')],
  body: Uint8List.fromList('a=1'.codeUnits),
  timeoutMs: timeoutMs,
);

void main() {
  test('sends as given, without following redirects', () async {
    late http.BaseRequest seen;
    final client = MockClient((request) async {
      seen = request;
      return http.Response(
        '',
        302,
        headers: {'location': '/vtop/content', 'content-length': '0'},
      );
    });

    final response = await sendPlatformHttp(client, _request());

    expect(seen.followRedirects, isFalse);
    expect(seen.headers['cookie'], 'JSESSIONID=a');
    expect((seen as http.Request).body, 'a=1');
    expect(response.errorKind, isNull);
    expect(response.status, 302);
    expect(response.headers, contains(('location', '/vtop/content')));
  });

  test('keeps each Set-Cookie line apart', () async {
    final client = MockClient(
      (_) async => http.Response(
        'ok',
        200,
        headers: {
          'set-cookie':
              'A=1; Expires=Wed, 21 Oct 2026 10:00:00 GMT; Path=/,B=2; Path=/',
        },
      ),
    );

    final response = await sendPlatformHttp(client, _request());

    final cookies = [
      for (final (name, value) in response.headers)
        if (name == 'set-cookie') value,
    ];
    expect(cookies, [
      'A=1; Expires=Wed, 21 Oct 2026 10:00:00 GMT; Path=/',
      'B=2; Path=/',
    ]);
  });

  test('reports failures instead of throwing', () async {
    final failing = MockClient((_) async => throw http.ClientException('no'));
    final slow = MockClient((_) => Completer<http.Response>().future);

    final failed = await sendPlatformHttp(failing, _request());
    final timedOut = await sendPlatformHttp(slow, _request(timeoutMs: 10));

    expect(failed.errorKind, PlatformHttpErrorKind.other);
    expect(timedOut.errorKind, PlatformHttpErrorKind.timeout);
  });
}
