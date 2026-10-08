import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';

void main() {
  const base = 'https://bridge.test';
  const cookies = 'JSESSIONID=abc';

  ({
    VtopBridgeAccountService service,
    List<http.Request> sent,
    MemoryBridgeAppSecretStore secrets,
  })
  serviceAnswering(int status, Object body, {String? secret}) {
    final sent = <http.Request>[];
    final secrets = MemoryBridgeAppSecretStore(secret);
    final client = MockClient((request) async {
      sent.add(request);
      return http.Response(
        jsonEncode(body),
        status,
        headers: {'content-type': 'application/json'},
      );
    });
    return (
      service: VtopBridgeAccountService(
        client: client,
        baseUrl: base,
        secrets: secrets,
      ),
      sent: sent,
      secrets: secrets,
    );
  }

  test('link sends the session, FCM token and credentials', () async {
    final rig = serviceAnswering(204, {});
    await rig.service.link(
      cookies: cookies,
      fcmToken: 'fcm-1',
      credentials: const {'username': 'u', 'password': 'p'},
    );
    final request = rig.sent.single;
    expect(request.url.toString(), '$base/v1/link');
    expect(jsonDecode(request.body), {
      'session': {'cookies': cookies},
      'fcmToken': 'fcm-1',
      'credentials': {'username': 'u', 'password': 'p'},
    });
  });

  test('link without credentials omits them', () async {
    final rig = serviceAnswering(204, {});
    await rig.service.link(cookies: cookies, fcmToken: 'fcm-1');
    expect(
      (jsonDecode(rig.sent.single.body) as Map).containsKey('credentials'),
      isFalse,
    );
  });

  test('createKey returns the key once with the MCP URL', () async {
    final rig = serviceAnswering(201, {
      'id': 'abcd1234',
      'key': 'vtm_${'0' * 64}',
      'mcpUrl': 'https://mcp.test/mcp',
    });
    final key = await rig.service.createKey(cookies: cookies, label: 'Laptop');
    expect(key.id, 'abcd1234');
    expect(key.key, startsWith('vtm_'));
    expect(key.mcpUrl, 'https://mcp.test/mcp');
    expect(key.agentUrl, 'https://mcp.test/mcp/${key.key}');
    expect(jsonDecode(rig.sent.single.body)['label'], 'Laptop');
  });

  test('account parses keys and flags', () async {
    final rig = serviceAnswering(200, {
      'registrationNumber': '22BCE0001',
      'linked': true,
      'savedCredentials': false,
      'phoneLinked': true,
      'thisPhone': true,
      'keys': [
        {
          'id': 'abcd1234',
          'label': 'Laptop',
          'createdAt': 1790000000,
          'lastUsedAt': null,
        },
      ],
    });
    final account = await rig.service.account(cookies: cookies);
    expect(account.registrationNumber, '22BCE0001');
    expect(account.linked, isTrue);
    expect(account.savedCredentials, isFalse);
    expect(account.phoneLinked, isTrue);
    expect(account.linkedHere, isTrue);
    expect(account.keys.single.label, 'Laptop');
    expect(account.keys.single.lastUsedAt, isNull);
    expect(
      account.keys.single.createdAt,
      DateTime.fromMillisecondsSinceEpoch(1790000000 * 1000, isUtc: true),
    );
  });

  test('management calls hit their endpoints', () async {
    final rig = serviceAnswering(204, {});
    await rig.service.revokeKey(cookies: cookies, id: 'abcd1234');
    await rig.service.updateFcmToken(cookies: cookies, fcmToken: 'fcm-2');
    await rig.service.forgetCredentials(cookies: cookies);
    await rig.service.deleteAccount(cookies: cookies);
    expect(rig.sent.map((r) => r.url.path), [
      '/v1/keys/revoke',
      '/v1/account/fcm-token',
      '/v1/account/forget-credentials',
      '/v1/account/delete',
    ]);
    expect(jsonDecode(rig.sent[0].body)['id'], 'abcd1234');
    expect(jsonDecode(rig.sent[1].body)['fcmToken'], 'fcm-2');
  });

  test('errors carry the bridge code', () async {
    final rig = serviceAnswering(409, {
      'code': 'key_limit',
      'error': 'too many',
    });
    await expectLater(
      rig.service.createKey(cookies: cookies, label: 'x'),
      throwsA(
        isA<BridgeAccountException>()
            .having((e) => e.code, 'code', 'key_limit')
            .having((e) => e.statusCode, 'statusCode', 409),
      ),
    );
  });

  test('FCM token sync only when linked and changed', () {
    expect(
      shouldSyncFcmToken(linked: false, lastSent: null, current: 'a'),
      isFalse,
    );
    expect(
      shouldSyncFcmToken(linked: true, lastSent: 'a', current: 'a'),
      isFalse,
    );
    expect(
      shouldSyncFcmToken(linked: true, lastSent: 'a', current: 'b'),
      isTrue,
    );
    expect(
      shouldSyncFcmToken(linked: true, lastSent: null, current: 'b'),
      isTrue,
    );
    expect(
      shouldSyncFcmToken(linked: true, lastSent: 'a', current: null),
      isFalse,
    );
    expect(
      shouldSyncFcmToken(linked: true, lastSent: 'a', current: ''),
      isFalse,
    );
  });

  test('link keeps the app secret and later calls send it', () async {
    final rig = serviceAnswering(200, {'appSecret': 'vta_new'});
    await rig.service.link(cookies: cookies, fcmToken: 'fcm-1');
    expect(await rig.secrets.read(), 'vta_new');
    await rig.service.revokeKey(cookies: cookies, id: 'abcd1234');
    expect(jsonDecode(rig.sent.last.body)['appSecret'], 'vta_new');
  });

  test('relinking presents the old secret', () async {
    final rig = serviceAnswering(200, {
      'appSecret': 'vta_new',
    }, secret: 'vta_old');
    await rig.service.link(cookies: cookies, fcmToken: 'fcm-1');
    expect(jsonDecode(rig.sent.single.body)['appSecret'], 'vta_old');
    expect(await rig.secrets.read(), 'vta_new');
  });

  test('deleteAccount forgets the app secret', () async {
    final rig = serviceAnswering(204, {}, secret: 'vta_old');
    await rig.service.deleteAccount(cookies: cookies);
    expect(await rig.secrets.read(), isNull);
  });

  test('an account linked from another install is not linked here', () async {
    final rig = serviceAnswering(200, {
      'registrationNumber': '22BCE0001',
      'linked': true,
      'thisPhone': false,
      'keys': [],
    });
    final account = await rig.service.account(cookies: cookies);
    expect(account.linked, isTrue);
    expect(account.linkedHere, isFalse);
  });

  test('account parses settings, with defaults when missing', () async {
    final rig = serviceAnswering(200, {
      'linked': true,
      'thisPhone': true,
      'settings': {
        'phoneWaitSecs': 45,
        'vaultTtlSecs': 86400,
        'alwaysUseVault': false,
      },
      'keys': [],
    });
    final settings = (await rig.service.account(cookies: cookies)).settings;
    expect(settings.phoneWaitSecs, 45);
    expect(settings.vaultTtlSecs, 86400);
    expect(settings.alwaysUseVault, isFalse);

    final bare = serviceAnswering(200, {'linked': true, 'keys': []});
    final defaults = (await bare.service.account(cookies: cookies)).settings;
    expect(defaults, const BridgeSettings());
    expect(defaults.phoneWaitSecs, 20);
    expect(defaults.vaultTtlSecs, isNull);
    expect(defaults.alwaysUseVault, isTrue);
  });

  test('updateSettings sends every field, never as null-dropped', () async {
    final rig = serviceAnswering(204, {}, secret: 'vta_s');
    await rig.service.updateSettings(
      cookies: cookies,
      settings: const BridgeSettings(alwaysUseVault: false),
    );
    final body = jsonDecode(rig.sent.single.body) as Map;
    expect(rig.sent.single.url.path, '/v1/account/settings');
    expect(body['appSecret'], 'vta_s');
    expect(body['settings'], {
      'alwaysUseVault': false,
      'phoneWaitSecs': 20,
      'vaultTtlSecs': null,
    });
  });

  test('settings copyWith can clear the expiry', () {
    const oneDay = BridgeSettings(vaultTtlSecs: 86400);
    expect(oneDay.copyWith(vaultTtlSecs: () => null).vaultTtlSecs, isNull);
    expect(oneDay.copyWith(phoneWaitSecs: 10).vaultTtlSecs, 86400);
  });

  test('only the offered phone waits are choices', () {
    expect(BridgeSettings.phoneWaitChoices, [10, 20, 45]);
  });
}
