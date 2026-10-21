import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/utils/app_http_client.dart';
import 'package:vitapmate/features/settings/presentation/pages/server_page.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/server_format.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/server_url_section.dart';

void main() {
  test('the connected-apps route opens the Server page', () {
    expect(serverPagePath, '/settings/server');
    expect(Paths.server, 'server');
  });

  test('the header names each Server page', () {
    expect(serverHeaderTitle('/settings/server'), 'Server');
    expect(
      serverHeaderTitle('/settings/server/data'),
      'Your data on the server',
    );
    expect(
      serverHeaderTitle('/settings/server/apps/chrome-extension'),
      'Browser extension',
    );
  });

  group('widgets', () {
    Future<void> pump(
      WidgetTester tester,
      Widget child, {
      Map<String, Object> prefs = const {},
      http.Client? client,
    }) async {
      SharedPreferencesAsyncPlatform.instance =
          InMemorySharedPreferencesAsync.withData(prefs);
      final container = ProviderContainer(
        overrides: [
          appHttpClientProvider.overrideWithValue(
            client ?? MockClient((_) async => http.Response('{}', 500)),
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
            home: Scaffold(body: SingleChildScrollView(child: child)),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    testWidgets('shows the default URL marked Default', (tester) async {
      await pump(tester, const ServerUrlSection());
      expect(find.text('https://vmate-api.aster0.dev'), findsOneWidget);
      expect(find.text('Default'), findsOneWidget);
    });

    Future<void> enterUrl(WidgetTester tester, String url) async {
      await tester.tap(find.text('https://vmate-api.aster0.dev'));
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), url);
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
    }

    testWidgets('saving an invalid URL shows the error and keeps the old URL', (
      tester,
    ) async {
      await pump(tester, const ServerUrlSection());
      await enterUrl(tester, 'https://h.example/v1');
      expect(find.text(serverUrlFormatError), findsOneWidget);
      expect(find.text('Switch servers?'), findsNothing);
    }, semanticsEnabled: false);

    testWidgets('saving a URL that fails the health check is refused', (
      tester,
    ) async {
      await pump(tester, const ServerUrlSection());
      await enterUrl(tester, 'https://h.example');
      expect(find.text(serverUnreachableError), findsOneWidget);
      expect(find.text('Switch servers?'), findsNothing);
    }, semanticsEnabled: false);

    testWidgets('changing the URL asks first', (tester) async {
      await pump(
        tester,
        const ServerUrlSection(),
        client: MockClient(
          (_) async => http.Response('{"service":"vtop-edge"}', 200),
        ),
      );
      await enterUrl(tester, 'https://h.example');
      expect(find.text('Switch servers?'), findsOneWidget);
    }, semanticsEnabled: false);
  });
}
