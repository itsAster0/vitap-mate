import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/features/settings/connection_page.dart';
import 'package:vitapmate/vtop/edge/edge_events.dart';
import 'package:vitapmate/vtop/route/connection_profile.dart';
import 'package:vitapmate/vtop/route/vtop_proxy.dart';
import 'package:vitapmate/vtop/route/vtop_udp.dart';

void main() {
  Future<ProviderContainer> settings(Map<String, Object> saved) async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.withData(saved);
    final container = ProviderContainer();
    addTearDown(container.dispose);
    await container.read(settingsProvider.future);
    return container;
  }

  Future<ConnectionProfile> profileOf(Map<String, Object> saved) async =>
      (await settings(saved)).read(connectionProfileProvider);

  test('the saved settings name their profile', () async {
    expect(await profileOf({}), ConnectionProfile.compress, reason: 'default');
    expect(
      await profileOf({vtopModeSettingKey: 'off'}),
      ConnectionProfile.phoneOnly,
    );
    expect(
      await profileOf({vtopModeSettingKey: 'server', vtopUdpSettingKey: true}),
      ConnectionProfile.server,
    );
    // Anything set by hand.
    expect(
      await profileOf({vtopModeSettingKey: 'server', vtopUdpSettingKey: false}),
      ConnectionProfile.custom,
    );
    expect(await profileOf({udpTimeout.key: 30}), ConnectionProfile.custom);
  });

  testWidgets('a profile saves its settings; a change by hand is Custom', (
    tester,
  ) async {
    final container = await settings({});
    // As at launch with FCM: the page holds the stream only while open.
    EdgeEventChannel.instance.alwaysOn = () async => false;
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          builder: (_, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(body: ConnectionPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Timeouts'), findsNothing);

    Future<void> choose(String title) async {
      await tester.tap(find.text(title));
      // Saving is real I/O.
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 200)),
      );
      await tester.pumpAndSettle();
    }

    await choose('Data saver');
    expect(container.read(connectionProfileProvider), ConnectionProfile.server);
    expect(container.read(vtopModeProvider), VtopServerMode.server);
    expect(container.read(vtopUdpProvider), isTrue);
    expect(
      find.text('Timeouts'),
      findsNothing,
      reason: 'Advanced is collapsed',
    );

    // Advanced shows what the profile set; a change there makes it Custom.
    expect(find.text('Custom: changed under Advanced'), findsNothing);
    await tester.tap(find.text('Advanced'));
    await tester.pumpAndSettle();
    expect(find.text('Timeouts'), findsOneWidget);
    await tester.runAsync(
      () => container
          .read(settingsProvider.future)
          .then((prefs) => prefs.setInt(udpTimeout.key, 30)),
    );
    container.invalidate(tunableProvider(udpTimeout));
    await tester.pumpAndSettle();
    expect(container.read(connectionProfileProvider), ConnectionProfile.custom);
    expect(find.text('Custom: changed under Advanced'), findsOneWidget);
  });
}
