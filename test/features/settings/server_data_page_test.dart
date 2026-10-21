import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/edge/server_status.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/features/settings/presentation/pages/server/server_data_page.dart';

void main() {
  testWidgets('lists what the server keeps, and what it never does', (
    tester,
  ) async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.withData({
          bridgeLinkedSettingKey: true,
        });
    final container = ProviderContainer(
      retry: (_, _) => null,
      overrides: [
        serverStatusProvider.overrideWith(
          (ref) async => ServerStatus(
            sessionSavedAt: DateTime.now().toUtc(),
            sessionSource: 'phone',
            phones: 2,
          ),
        ),
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
          home: const Scaffold(body: ServerDataPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Sealed password'), findsOneWidget);
    expect(find.text('None'), findsOneWidget);
    expect(find.textContaining('made by this phone'), findsOneWidget);
    expect(find.text(neverKeptText), findsOneWidget);
  });
}
