import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/app/settings.dart';

void main() {
  Future<ProviderContainer> container(Map<String, Object> prefs) async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.withData(prefs);
    final container = ProviderContainer();
    addTearDown(container.dispose);
    await container.read(settingsProvider.future);
    return container;
  }

  test('skip counts are off until turned on', () async {
    expect((await container({})).read(showSkipAdviceProvider), isFalse);
    final on = await container({showSkipAdviceSettingKey: true});
    expect(on.read(showSkipAdviceProvider), isTrue);
  });
}
