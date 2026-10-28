import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/vtop/route/vtop_proxy.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/vtop/backend/vtop_backend_provider.dart';

void main() {
  Future<String> backendName(Map<String, Object> prefs) async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.withData(prefs);
    final container = ProviderContainer();
    addTearDown(container.dispose);
    await container.read(settingsProvider.future);
    return container.read(vtopBackendProvider).name;
  }

  // The mode is read at each call (ServerVtopBackend), so one backend
  // serves every setting.
  test('one backend reads VTOP for every mode', () async {
    expect(await backendName({}), 'phone or server');
    expect(
      await backendName({
        bridgeLinkedSettingKey: true,
        vtopModeSettingKey: 'server',
      }),
      'phone or server',
    );
    expect(
      await backendName({onDeviceModeSettingKey: true}),
      'phone or server',
    );
  });
}
