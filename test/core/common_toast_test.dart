import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:vitapmate/app/widgets/common_toast.dart';

void main() {
  testWidgets('a long message is cut short, not overflowing', (tester) async {
    tester.view.physicalSize = const Size(1080, 2400);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        builder: (_, child) => FTheme(
          data: FTheme.neutral.dark.touch,
          child: FToaster(child: child!),
        ),
        home: Builder(
          builder: (context) => Scaffold(
            body: TextButton(
              onPressed: () =>
                  dispToast(context, 'Could not switch', 'word ' * 1000),
              child: const Text('go'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('go'));
    await tester.pump(const Duration(seconds: 1));
    expect(find.text('Could not switch'), findsOneWidget);
    final message = tester.getSize(find.textContaining('word word'));
    expect(message.height, lessThan(120));
  });
}
