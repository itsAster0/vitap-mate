import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart';
import 'package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/vtop/email_otp/google_email_oauth_service.dart';
import 'package:vitapmate/features/developer/debug_page.dart';

void main() {
  Future<void> pump(WidgetTester tester, {required bool gmailReady}) async {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.empty();
    final container = ProviderContainer(
      retry: (_, _) => null,
      overrides: [
        emailOtpReadyProvider.overrideWith((ref) async => gmailReady),
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
          home: const Scaffold(body: DebugPage()),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> expectRow(WidgetTester tester, String title) async {
    await tester.scrollUntilVisible(
      find.text(title),
      100,
      scrollable: find.byType(Scrollable).first,
    );
    expect(find.text(title), findsOneWidget, reason: title);
  }

  testWidgets('every debug option has a row', (tester) async {
    await pump(tester, gmailReady: true);
    for (final title in [
      'VTOP Session Reuse',
      'Copy Token',
      'Copy Saved Cookies',
      'Clear Saved Cookies',
      'Drop Latest History Entry',
      'Clear Attendance',
      'Test Latest OTP Email',
      'Logs',
    ]) {
      await expectRow(tester, title);
    }
  });

  testWidgets('the OTP email test needs Gmail set up', (tester) async {
    await pump(tester, gmailReady: false);
    await expectRow(tester, 'Logs');
    expect(find.text('Test Latest OTP Email'), findsNothing);
  });
}
