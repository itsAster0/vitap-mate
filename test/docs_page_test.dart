import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/providers/theme_provider.dart';
import 'package:vitapmate/core/router/paths.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/data/docs_repository.dart';
import 'package:vitapmate/features/docs/presentation/pages/docs_page.dart';
import 'package:vitapmate/features/docs/presentation/pages/document_viewer_page.dart';
import 'package:vitapmate/features/docs/presentation/providers/docs_provider.dart';
import 'package:vitapmate/features/docs/presentation/providers/docs_sort_provider.dart';
import 'package:vitapmate/features/docs/presentation/widgets/doc_card.dart';
import 'support/docs_test_storage.dart';

class _Theme extends ThemeModeController {
  _Theme(this.mode);
  final ThemeMode mode;
  @override
  ThemeMode build() => mode;
}

class _Repository extends DocsRepository {
  _Repository(super.storage);
  @override
  Future<String?> storedFilePathOf(DocWindow window) async => null;
}

void main() {
  for (final dark in [false, true]) {
    testWidgets(
      'search, sorting and returning from viewer in ${dark ? 'dark' : 'light'} theme',
      (tester) async {
        tester.view.reset();
        tester.view.physicalSize = const Size(360, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final storage = DocsTestStorage();
        final repo = _Repository(storage);
        final notes = await repo.importFile(
          sourcePath: '/notes.pdf',
          name: 'Notes',
        );
        await repo.importFile(sourcePath: '/zebra.pdf', name: 'Zebra');
        await repo.touchLastOpened(notes.id, openedAt: 123);
        final container = ProviderContainer(
          overrides: [
            jsonFileStorageProvider.overrideWith((ref) async => storage),
            docsRepositoryProvider.overrideWith((ref) async => repo),
            themeProvider.overrideWith(
              () => _Theme(dark ? ThemeMode.dark : ThemeMode.light),
            ),
          ],
        );
        addTearDown(container.dispose);
        final router = GoRouter(
          routes: [
            GoRoute(
              path: '/',
              builder: (_, _) => const Scaffold(body: DocsPage()),
            ),
            GoRoute(
              path: '/viewer',
              name: Paths.docView,
              builder: (_, state) => Scaffold(
                appBar: AppBar(),
                body: DocumentViewerPage(doc: state.extra! as DocWindow),
              ),
            ),
          ],
        );
        addTearDown(router.dispose);
        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: container,
            child: MaterialApp.router(
              routerConfig: router,
              builder: (_, child) => FTheme(
                data: dark
                    ? FTheme.neutral.dark.touch
                    : FTheme.neutral.light.touch,
                child: FToaster(child: child!),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.text('Continue viewing'), findsOneWidget);
        expect(
          (await repo.list()).firstWhere((d) => d.id == notes.id).lastOpenedAt,
          123,
        );
        expect(find.text('Sort'), findsOneWidget);
        expect(
          tester.getCenter(find.byType(FTextField)).dy,
          tester.getCenter(find.text('Sort')).dy,
        );

        await tester.enterText(find.byType(TextField), '  nOt  ');
        await tester.pumpAndSettle();
        expect(find.text('Continue viewing'), findsNothing);
        expect(find.text('1 of 3 documents'), findsOneWidget);
        expect(tester.widget<DocCard>(find.byType(DocCard)).doc.id, notes.id);
        await tester.tap(find.byType(DocCard));
        await tester.pumpAndSettle();
        expect(
          (await repo.list()).firstWhere((d) => d.id == notes.id).lastOpenedAt,
          greaterThan(123),
        );
        router.pop();
        await tester.pumpAndSettle();
        expect(
          tester.widget<TextField>(find.byType(TextField)).controller!.text,
          '  nOt  ',
        );
        expect(find.text('1 of 3 documents'), findsOneWidget);

        await tester.enterText(find.byType(TextField), 'nothing matches');
        await tester.pumpAndSettle();
        expect(find.text('No documents found'), findsOneWidget);
        expect(find.text('0 of 3 documents'), findsOneWidget);
        await tester.tap(find.text('Clear search'));
        await tester.pumpAndSettle();
        expect(find.text('Continue viewing'), findsOneWidget);

        await tester.enterText(find.byType(TextField), 'e');
        await tester.pumpAndSettle();
        await tester.tap(find.text('Sort'));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Name Z–A'));
        await tester.pumpAndSettle();
        expect(find.text('Sort'), findsOneWidget);
        expect(
          tester
              .widgetList<DocCard>(find.byType(DocCard))
              .map((c) => c.doc.name),
          ['Zebra', 'Notes', 'Mess Menu'],
        );
        expect(
          storage.data[DocsSortNotifier.storageKey]?['sort'],
          'nameDescending',
        );
        await tester.tap(find.byTooltip('Clear search'));
        await tester.pumpAndSettle();
        expect(find.text('Continue viewing'), findsOneWidget);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
        await tester.pumpAndSettle();
      },
      semanticsEnabled: false,
    );
  }
}
