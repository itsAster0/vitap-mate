import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/data/docs_repository.dart';
import 'package:vitapmate/features/docs/domain/docs_query.dart';
import 'package:vitapmate/features/docs/presentation/providers/docs_sort_provider.dart';
import 'support/docs_test_storage.dart';

ProviderContainer containerFor(DocsTestStorage storage) => ProviderContainer(
  overrides: [jsonFileStorageProvider.overrideWith((ref) async => storage)],
);

void main() {
  test(
    'missing, invalid and unreadable preferences use recently opened',
    () async {
      for (final value in [null, 'invalid', 7]) {
        final storage = DocsTestStorage();
        storage.data[DocsSortNotifier.storageKey] = {'sort': value};
        final container = containerFor(storage);
        addTearDown(container.dispose);
        expect(
          await container.read(docsSortProvider.future),
          DocSort.recentlyOpened,
        );
      }
      final container = containerFor(DocsTestStorage()..failRead = true);
      addTearDown(container.dispose);
      expect(
        await container.read(docsSortProvider.future),
        DocSort.recentlyOpened,
      );
    },
  );

  test(
    'sort survives recreation and rapid choices save the final selection',
    () async {
      final storage = DocsTestStorage();
      final container = containerFor(storage);
      addTearDown(container.dispose);
      await container.read(docsSortProvider.future);
      final notifier = container.read(docsSortProvider.notifier);
      await Future.wait([
        notifier.select(DocSort.nameAscending),
        notifier.select(DocSort.recentlyAdded),
        notifier.select(DocSort.nameDescending),
      ]);
      final restored = containerFor(storage);
      addTearDown(restored.dispose);
      expect(
        await restored.read(docsSortProvider.future),
        DocSort.nameDescending,
      );
      final otherUser = containerFor(DocsTestStorage());
      addTearDown(otherUser.dispose);
      expect(
        await otherUser.read(docsSortProvider.future),
        DocSort.recentlyOpened,
      );
    },
  );

  test('failed saves retain selection and subsequent saves recover', () async {
    final storage = DocsTestStorage();
    final container = containerFor(storage);
    addTearDown(container.dispose);
    await container.read(docsSortProvider.future);
    storage.failWrite = true;
    final notifier = container.read(docsSortProvider.notifier);
    await expectLater(notifier.select(DocSort.nameAscending), throwsStateError);
    expect(container.read(docsSortProvider).value, DocSort.nameAscending);
    storage.failWrite = false;
    await notifier.select(DocSort.recentlyAdded);
    expect(storage.data[DocsSortNotifier.storageKey]?['sort'], 'recentlyAdded');
  });

  test('starts empty and the mess menu flag only changes the view', () async {
    final repo = DocsRepository(DocsTestStorage());
    expect(await repo.list(), isEmpty);
    final sheet = await repo.importFile(sourcePath: '/menu.xlsx', name: 'Menu');
    await repo.setMessMenu(sheet.id, true);
    var stored = (await repo.list()).single;
    expect(stored.asMessMenu, isTrue);
    expect(stored.fileName, sheet.fileName);
    expect(stored.kind, DocKind.spreadsheet);
    await repo.setMessMenu(sheet.id, false);
    stored = (await repo.list()).single;
    expect(stored.asMessMenu, isFalse);
    expect(stored.fileName, sheet.fileName);
    await repo.remove(sheet.id);
    expect(await repo.list(), isEmpty);
  });

  test('imports do not count as opens', () async {
    final repo = DocsRepository(DocsTestStorage());
    final imported = await repo.importFile(
      sourcePath: '/notes.pdf',
      name: 'Notes',
    );
    expect(imported.lastOpenedAt, isNull);
    expect(imported.addedAt, greaterThan(0));
    await repo.touchLastOpened(imported.id, openedAt: 123);
    await repo.rename(imported.id, 'New name');
    await repo.saveScrollState(imported.id, scale: 2, offsetX: 4, offsetY: 8);
    final updated = (await repo.list()).firstWhere((d) => d.id == imported.id);
    expect(updated.lastOpenedAt, 123);
    expect(updated.addedAt, imported.addedAt);
    expect(updated.name, 'New name');
  });
}
