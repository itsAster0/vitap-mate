import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';
import 'package:vitapmate/vtop/edge/edge_device.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';

void main() {
  void onDeviceMode(bool on) {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.withData({onDeviceModeSettingKey: on});
  }

  ({http.Client client, List<http.Request> sent}) recorder() {
    final sent = <http.Request>[];
    return (
      client: MockClient((request) async {
        sent.add(request);
        return http.Response('{}', 200);
      }),
      sent: sent,
    );
  }

  test('reads the stored switch', () async {
    onDeviceMode(true);
    expect(await isOnDeviceMode(), isTrue);
    onDeviceMode(false);
    expect(await isOnDeviceMode(), isFalse);
  });

  test('edge calls are refused without a request', () async {
    onDeviceMode(true);
    final rig = recorder();
    await expectLater(
      edgePost(rig.client, 'https://edge.test', '/v1/devices', {}),
      throwsA(
        isA<EdgeException>().having(
          (e) => e.code,
          'code',
          OnDeviceModeException.code,
        ),
      ),
    );
    expect(rig.sent, isEmpty);
  });

  test('edge calls go out when the mode is off', () async {
    onDeviceMode(false);
    final rig = recorder();
    await edgePost(rig.client, 'https://edge.test', '/v1/devices', {});
    expect(rig.sent, hasLength(1));
  });

  test('bridge calls are refused without a request', () async {
    onDeviceMode(true);
    final rig = recorder();
    final service = VtopBridgeAccountService(
      client: rig.client,
      baseUrl: 'https://bridge.test',
      secrets: MemoryBridgeAppSecretStore('secret'),
      keys: MemoryBridgeKeyStash(),
    );
    await expectLater(
      service.updateSemester(cookies: 'JSESSIONID=a', semesterId: 'AP1'),
      throwsA(
        isA<BridgeAccountException>().having(
          (e) => e.code,
          'code',
          OnDeviceModeException.code,
        ),
      ),
    );
    await service.reportCredentialsRejected(registrationNumber: '23BCE1');
    expect(rig.sent, isEmpty);
  });
}
