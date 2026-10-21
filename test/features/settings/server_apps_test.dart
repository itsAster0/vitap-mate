import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/utils/edge/server_status.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/features/settings/presentation/pages/server/server_apps_page.dart';
import 'package:vitapmate/features/settings/presentation/providers/server_account.dart';

class _Api implements ServerAccountApi {
  @override
  Future<BridgeAccount> account() async => BridgeAccount(
    registrationNumber: '22BCE0001',
    linked: true,
    savedCredentials: false,
    phoneLinked: true,
    thisPhone: true,
    semesterId: 'S',
    keys: [
      BridgeKeyInfo(
        id: 'k1',
        label: 'laptop',
        createdAt: DateTime.utc(2026),
        lastUsedAt: DateTime.now().toUtc(),
      ),
    ],
  );

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  final now = DateTime.now().toUtc();

  Future<void> pump(
    WidgetTester tester,
    Widget page, {
    ServerStatus status = const ServerStatus(),
  }) async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.withData({bridgeLinkedSettingKey: true});
    final container = ProviderContainer(
      retry: (_, _) => null,
      overrides: [
        serverAccountApiProvider.overrideWithValue(_Api()),
        serverStatusProvider.overrideWith((ref) async => status),
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
          home: Scaffold(body: page),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('keys show their recent uses by app', (tester) async {
    await pump(
      tester,
      const ServerAppsPage(),
      status: ServerStatus(
        keys: [
          KeyActivity(
            id: 'k1',
            label: 'laptop',
            lastUsedAt: now,
            uses: [
              KeyUse(at: now, via: 'mcp'),
              KeyUse(
                at: now.subtract(const Duration(hours: 3)),
                via: 'extension',
              ),
            ],
          ),
        ],
      ),
    );
    expect(find.text('laptop'), findsOneWidget);
    expect(
      find.textContaining('MCP just now · Extension 3 h ago'),
      findsOneWidget,
    );
  });
}
