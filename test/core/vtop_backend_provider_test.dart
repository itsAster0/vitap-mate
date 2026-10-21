import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/core/providers/on_device_mode.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/providers/vtop_proxy.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/core/vtop_backend/vtop_backend_provider.dart';

void main() {
  Future<String> backendName(Map<String, Object> prefs) async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.withData(prefs);
    final container = ProviderContainer();
    addTearDown(container.dispose);
    await container.read(settingsProvider.future);
    return container.read(vtopBackendProvider).name;
  }

  test('VTOP is always read on the phone, linked, proxied or not', () async {
    expect(await backendName({}), 'local');
    expect(
      await backendName({
        bridgeLinkedSettingKey: true,
        vtopProxySettingKey: true,
      }),
      'local',
    );
    expect(await backendName({onDeviceModeSettingKey: true}), 'local');
  });
}
