import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:vitapmate/core/utils/edge/server_status.dart';

void main() {
  test('health reads version and region', () async {
    final health = await checkHealth(
      'https://e',
      MockClient(
        (_) async => http.Response(
          '{"service":"vtop-edge","status":"ok","version":"0.5.0","colo":"SIN"}',
          200,
        ),
      ),
    );
    expect(health.ok, isTrue);
    expect(health.version, '0.5.0');
    expect(health.colo, 'SIN');
    expect(health.rttMs, greaterThanOrEqualTo(0));
  });

  test('something that is not the edge is not ok', () async {
    final health = await checkHealth(
      'https://e',
      MockClient((_) async => http.Response('{"service":"other"}', 200)),
    );
    expect(health.ok, isFalse);
  });

  ServerStatusService service(MockClient client) => ServerStatusService(
    client: client,
    identity: () async => (appSecret: 'vta_x', registrationNumber: '22BCE0001'),
    serverUrl: () async => 'https://e',
  );

  test('status parses every field', () async {
    final status = await service(
      MockClient((request) async {
        expect(request.url.path, '/v1/app/status');
        expect(jsonDecode(request.body), {
          'appSecret': 'vta_x',
          'registrationNumber': '22BCE0001',
        });
        return http.Response(
          jsonEncode({
            'sessionSavedAt': 2000,
            'sessionSource': 'server',
            'forwarding': 'vtop-otp-a@aster0.dev',
            'phones': 2,
            'signIns': [
              {'at': 3000, 'by': 'phone', 'ok': false, 'reason': 'x'},
            ],
            'keys': [
              {
                'id': 'k1',
                'label': 'laptop',
                'lastUsedAt': 4000,
                'uses': [
                  {'at': 4000, 'via': 'mcp'},
                ],
              },
            ],
          }),
          200,
        );
      }),
    ).status();
    DateTime t(int s) =>
        DateTime.fromMillisecondsSinceEpoch(s * 1000, isUtc: true);
    expect(status.sessionSavedAt, t(2000));
    expect(status.sessionSource, 'server');
    expect(status.forwarding, 'vtop-otp-a@aster0.dev');
    expect(status.phones, 2);
    expect(status.signIns.single.byServer, isFalse);
    expect(status.signIns.single.ok, isFalse);
    expect(status.signIns.single.reason, 'x');
    expect(status.keys.single.label, 'laptop');
    expect(status.keys.single.uses.single.via, 'mcp');
    expect(status.keys.single.lastUsedAt, t(4000));
  });

  test('not linked is its own error', () async {
    await expectLater(
      service(
        MockClient(
          (_) async => http.Response('{"code":"not_linked","error":"x"}', 403),
        ),
      ).status(),
      throwsA(isA<ServerNotLinked>()),
    );
  });

  test(
    'a secret the phone cannot read is not the server unlinking it',
    () async {
      // The phone would forget the link on ServerNotLinked; a secure-storage
      // read that fails says nothing about the server.
      await expectLater(
        ServerStatusService(
          client: MockClient((_) async => http.Response('{}', 200)),
          identity: () async => null,
          serverUrl: () async => 'https://e',
        ).status(),
        throwsA(isA<ServerIdentityUnavailable>()),
      );
    },
  );

  test('clear session posts the identity', () async {
    final paths = <String>[];
    await service(
      MockClient((request) async {
        paths.add(request.url.path);
        return http.Response('{"ok":true}', 200);
      }),
    ).clearSession();
    expect(paths, ['/v1/app/session/clear']);
  });
}
