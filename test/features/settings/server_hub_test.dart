import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:vitapmate/vtop/edge/server_status.dart';
import 'package:vitapmate/vtop/vtop_bridge_account_service.dart';
import 'package:vitapmate/features/server/server_page.dart';
import 'package:vitapmate/features/server/widgets/status_card.dart';

const healthy = ServerHealth(
  ok: true,
  rttMs: 120,
  version: '0.5.0',
  colo: 'SIN',
);
const slow = ServerHealth(ok: true, rttMs: 1500);
const down = ServerHealth(ok: false, rttMs: 8000);

void main() {
  group('statusCardState', () {
    CardState state({bool linked = true, ServerHealth? health = healthy}) =>
        statusCardState(linked: linked, health: health);

    test('each state', () {
      expect(state(linked: false), CardState.notLinked);
      expect(state(health: down), CardState.unreachable);
      expect(state(health: slow), CardState.slow);
      expect(state(), CardState.connected);
      expect(state(health: null), CardState.checking);
    });
  });

  group('hub', () {
    Future<void> pump(
      WidgetTester tester, {
      Map<String, Object> prefs = const {},
      ServerHealth health = healthy,
      Object status = const ServerStatus(),
    }) async {
      SharedPreferencesAsyncPlatform.instance =
          InMemorySharedPreferencesAsync.withData(prefs);
      final container = ProviderContainer(
        retry: (_, _) => null,
        overrides: [
          appHttpClientProvider.overrideWithValue(
            MockClient((_) async => http.Response('{}', 500)),
          ),
          serverHealthProvider.overrideWith((ref) async => health),
          serverStatusProvider.overrideWith((ref) async {
            if (status is ServerStatus) return status;
            throw status;
          }),
        ],
      );
      addTearDown(container.dispose);
      await container.read(settingsProvider.future);
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: container,
          child: MaterialApp(
            builder: (_, child) => FTheme(
              data: FTheme.neutral.light.touch,
              child: FToaster(child: child!),
            ),
            home: const Scaffold(body: ServerPage()),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    const linked = {bridgeLinkedSettingKey: true};

    testWidgets('a linked phone is connected', (tester) async {
      await pump(tester, prefs: linked);
      expect(find.text('Connected'), findsOneWidget);
      expect(find.textContaining('120 ms'), findsOneWidget);
    });

    testWidgets('a phone the server forgot is not linked', (tester) async {
      await pump(tester, prefs: linked, status: const ServerNotLinked());
      expect(find.text('Not linked'), findsOneWidget);
      expect(find.text('Link this phone'), findsOneWidget);
    });

    testWidgets('every area has a row', (tester) async {
      await pump(tester, prefs: linked);
      // The VTOP mode is one row until opened, then every mode.
      expect(find.text('Phone only'), findsOneWidget);
      expect(find.text('Fetch on the server'), findsNothing);
      await tester.tap(find.text('Phone only'));
      await tester.pumpAndSettle();
      for (final mode in [
        'Compress through the server',
        'Fetch on the server',
      ]) {
        expect(find.text(mode), findsOneWidget, reason: mode);
      }
      for (final title in [
        'Connected apps',
        'Linked phones',
        'Server address',
        'Your data on the server',
      ]) {
        await tester.scrollUntilVisible(
          find.text(title),
          100,
          scrollable: find.byType(Scrollable).first,
        );
        expect(find.text(title), findsOneWidget, reason: title);
      }
      // The server modes are gone.
      expect(find.text('How VTOP is reached'), findsNothing);
      expect(find.text('Sign-in'), findsNothing);
    });

    testWidgets('on-device mode locks the page with one turn-off button', (
      tester,
    ) async {
      await pump(tester, prefs: {...linked, onDeviceModeSettingKey: true});
      expect(find.text('Turn off On-device Mode'), findsOneWidget);
      expect(find.text('Connected apps'), findsNothing);
    });
  });
}
