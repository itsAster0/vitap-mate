import 'package:flutter_test/flutter_test.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
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

  test('imports and preset filling do not count as opens', () async {
    final repo = DocsRepository(DocsTestStorage());
    final imported = await repo.importFile(
      sourcePath: '/notes.pdf',
      name: 'Notes',
    );
    expect(imported.lastOpenedAt, isNull);
    expect(imported.addedAt, greaterThan(0));
    final preset = (await repo.list()).firstWhere((d) => d.isPreset);
    final filled = (await repo.fillPreset(preset.id, '/menu.pdf'))!;
    expect(filled.addedAt, greaterThan(0));
    expect(filled.lastOpenedAt, isNull);
    await repo.touchLastOpened(imported.id, openedAt: 123);
    await repo.rename(imported.id, 'New name');
    await repo.saveScrollState(imported.id, scale: 2, offsetX: 4, offsetY: 8);
    final updated = (await repo.list()).firstWhere((d) => d.id == imported.id);
    expect(updated.lastOpenedAt, 123);
    expect(updated.addedAt, imported.addedAt);
    expect(updated.name, 'New name');
  });
}
