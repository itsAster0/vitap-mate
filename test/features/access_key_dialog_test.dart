import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/core/utils/vtop_bridge_account_service.dart';
import 'package:vitapmate/features/settings/presentation/widgets/server_sections/key_dialogs.dart';

const _key = BridgeKey(
  id: 'k1',
  key: 'vtm_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef',
  mcpUrl: 'https://vmate-api.aster0.dev/mcp',
);

void main() {
  String? copied;

  setUp(() {
    copied = null;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(SystemChannels.platform, (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String;
          }
          return null;
        });
  });

  /// Scrolls [text] into view in the sheet, then taps it.
  Future<void> tapText(WidgetTester tester, String text) async {
    await tester.ensureVisible(find.text(text));
    await tester.pumpAndSettle();
    await tester.tap(find.text(text));
    await tester.pumpAndSettle();
  }

  Future<void> open(WidgetTester tester, {required bool fresh}) async {
    // A small phone.
    tester.view.physicalSize = const Size(320, 640);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        builder: (_, child) => FTheme(
          data: FTheme.neutral.light.touch,
          child: FToaster(child: child!),
        ),
        home: Builder(
          builder: (context) => Center(
            child: FButton(
              onPress: () => showAccessKey(context, _key, fresh: fresh),
              child: const Text('Show'),
            ),
          ),
        ),
      ),
    );
    await tapText(tester, 'Show');
  }

  testWidgets('a new key shows the key and how to see it again', (
    tester,
  ) async {
    await open(tester, fresh: true);
    expect(find.text('Your key is ready'), findsOneWidget);
    // Three quarters of the 640 px screen.
    expect(tester.getSize(find.byType(SafeArea).last).height, closeTo(480, 1));
    expect(find.text(_key.key), findsOneWidget);
    expect(find.textContaining('long-press it in the list'), findsOneWidget);
    await tester.tap(find.text(_key.key));
    await tester.pump();
    expect(copied, _key.key);
    expect(find.text('Copied'), findsOneWidget);
    await tester.pump(const Duration(seconds: 2));
    await tester.pumpAndSettle();
    expect(find.text('Copied'), findsNothing);
  });

  testWidgets('each AI agent setup copies what it needs, without overflow', (
    tester,
  ) async {
    await open(tester, fresh: false);
    expect(find.text('Access key'), findsWidgets);
    await tapText(tester, 'AI agent');

    await tapText(tester, _key.claudeCodeCommand);
    expect(copied, _key.claudeCodeCommand);

    await tapText(tester, 'Other agents');
    await tapText(tester, _key.authHeader);
    expect(copied, _key.authHeader);
    await tapText(tester, _key.mcpUrl);
    expect(copied, _key.mcpUrl);

    await tapText(tester, 'URL only');
    await tapText(tester, _key.agentUrl);
    expect(copied, _key.agentUrl);
    expect(find.textContaining('keep the link as secret'), findsOneWidget);

    await tapText(tester, 'Done');
    expect(find.text('Access key'), findsNothing);
  });
}
