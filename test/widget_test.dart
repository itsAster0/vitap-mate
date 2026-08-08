import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/features/more/presentation/pages/chrome_extension_page.dart';

void main() {
  testWidgets('Chrome extension guide explains setup and uses Token wording', (
    tester,
  ) async {
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          builder: (context, child) => FTheme(
            data: FTheme.neutral.light.touch,
            child: FToaster(child: child!),
          ),
          home: const Scaffold(body: ChromeExtensionPage()),
        ),
      ),
    );

    expect(find.text('Copy Token'), findsOneWidget);
    expect(find.text('Reset Token'), findsOneWidget);
    expect(find.text('Open GitHub release'), findsOneWidget);
    expect(find.text('Download extension.zip'), findsOneWidget);
    expect(find.text('Open Chrome extensions'), findsOneWidget);
    expect(find.textContaining('FCM'), findsNothing);
    expect(find.textContaining('FMC'), findsNothing);
  });
}
