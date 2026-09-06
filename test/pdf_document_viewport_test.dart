import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/features/docs/presentation/widgets/pdf_document_viewport.dart';

void main() {
  testWidgets('embedded PDF pinch wins over the surrounding page scroll', (
    tester,
  ) async {
    final transform = TransformationController();
    final scroll = ScrollController();
    final outerScroll = ScrollController();
    addTearDown(transform.dispose);
    addTearDown(scroll.dispose);
    addTearDown(outerScroll.dispose);
    var previewActive = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) => CustomScrollView(
              controller: outerScroll,
              physics: previewActive
                  ? const NeverScrollableScrollPhysics()
                  : const AlwaysScrollableScrollPhysics(),
              slivers: [
                SliverToBoxAdapter(
                  child: Listener(
                    onPointerDown: (_) => setState(() => previewActive = true),
                    onPointerUp: (_) => setState(() => previewActive = false),
                    onPointerCancel: (_) =>
                        setState(() => previewActive = false),
                    child: SizedBox(
                      height: 310,
                      child: PdfDocumentViewport(
                        transform: transform,
                        scroll: scroll,
                        pageCount: 30,
                        onInteractionEnd: () {},
                        pageBuilder: (_, index) =>
                            SizedBox(height: 500, child: Text('Page $index')),
                      ),
                    ),
                  ),
                ),
                const SliverToBoxAdapter(child: SizedBox(height: 1500)),
              ],
            ),
          ),
        ),
      ),
    );

    final first = await tester.startGesture(const Offset(300, 200), pointer: 1);
    await tester.pump();
    await first.moveBy(const Offset(0, -40));
    await tester.pump();
    await first.moveBy(const Offset(0, -40));
    await tester.pump();
    expect(scroll.offset, greaterThan(0));
    final second = await tester.startGesture(
      const Offset(440, 120),
      pointer: 2,
    );
    await tester.pump();
    for (var i = 0; i < 2; i++) {
      await first.moveBy(const Offset(-30, -15));
      await second.moveBy(const Offset(30, 15));
      await tester.pump();
    }
    expect(transform.value.getMaxScaleOnAxis(), greaterThan(1.2));
    expect(outerScroll.offset, 0);
    await second.up();
    await first.up();
    await tester.pumpAndSettle();

    // Scrolling outside the preview must still move the docs page.
    await tester.dragFrom(const Offset(400, 500), const Offset(0, -150));
    await tester.pumpAndSettle();
    expect(outerScroll.offset, greaterThan(0));
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('pinch zoom works after one finger starts scrolling', (
    tester,
  ) async {
    final transform = TransformationController();
    final scroll = ScrollController();
    addTearDown(transform.dispose);
    addTearDown(scroll.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: PdfDocumentViewport(
            transform: transform,
            scroll: scroll,
            pageCount: 30,
            onInteractionEnd: () {},
            pageBuilder: (_, index) =>
                SizedBox(height: 500, child: Text('Page $index')),
          ),
        ),
      ),
    );

    final first = await tester.startGesture(const Offset(300, 350), pointer: 1);
    await first.moveBy(const Offset(0, -80));
    await tester.pump();
    await first.moveBy(const Offset(0, -80));
    await tester.pump();
    expect(scroll.offset, greaterThan(0));

    final second = await tester.startGesture(
      const Offset(450, 190),
      pointer: 2,
    );
    await tester.pump();
    await first.moveBy(const Offset(-40, -30));
    await second.moveBy(const Offset(40, 30));
    await tester.pump();
    await first.moveBy(const Offset(-40, -30));
    await second.moveBy(const Offset(40, 30));
    await tester.pump();
    expect(transform.value.getMaxScaleOnAxis(), greaterThan(1.2));

    final zoomedScale = transform.value.getMaxScaleOnAxis();
    await first.moveBy(const Offset(30, 20));
    await second.moveBy(const Offset(-30, -20));
    await tester.pump();
    expect(transform.value.getMaxScaleOnAxis(), lessThan(zoomedScale));
    await first.up();
    await second.up();
    await tester.pumpAndSettle();

    final previousOffset = scroll.offset;
    await tester.drag(find.byType(PdfDocumentViewport), const Offset(0, -300));
    await tester.pumpAndSettle();
    expect(scroll.offset, greaterThan(previousOffset));
    expect(find.text('Page 29'), findsNothing);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
  });
}
