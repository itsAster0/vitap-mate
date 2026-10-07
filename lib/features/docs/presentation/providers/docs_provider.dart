import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/storage/json_file_storage_provider.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/data/docs_repository.dart';
import 'package:vitapmate/features/docs/data/download_to_docs.dart';
import 'package:vitapmate/features/docs/data/spreadsheet_tables.dart';
import 'package:vitapmate/features/docs/domain/mess_menu.dart';

Future<String?> pickDocPath() async {
  final file = await FilePicker.pickFile(
    type: FileType.custom,
    allowedExtensions: [
      'pdf',
      'png',
      'jpg',
      'jpeg',
      'webp',
      'gif',
      'bmp',
      'xlsx',
      'xlsm',
      'xls',
      'ods',
      'xml',
      'html',
      'htm',
      'txt',
      'json',
      'md',
      'csv',
    ],
  );
  return file?.path;
}

Future<MessMenu> loadMessMenu(DocsRepository repo, DocWindow doc) async {
  final path = await repo.storedFilePathOf(doc);
  if (path == null) throw StateError('Document file is missing.');
  return compute(_parseMessMenuBytes, await File(path).readAsBytes());
}

MessMenu _parseMessMenuBytes(Uint8List bytes) =>
    parseMessMenu(decodeSpreadsheetTables(bytes));

final docsRepositoryProvider = FutureProvider<DocsRepository>((ref) async {
  final storage = await ref.watch(jsonFileStorageProvider.future);
  return DocsRepository(storage);
});

final docsRegistryProvider =
    AsyncNotifierProvider<DocsRegistryNotifier, List<DocWindow>>(
      DocsRegistryNotifier.new,
    );

final activeDocumentTitleProvider =
    NotifierProvider<ActiveDocumentTitle, String?>(ActiveDocumentTitle.new);

class ActiveDocumentTitle extends Notifier<String?> {
  @override
  String? build() => null;

  void show(String title) => state = title;

  void clear(String title) {
    if (state == title) state = null;
  }
}

class DocsRegistryNotifier extends AsyncNotifier<List<DocWindow>> {
  @override
  Future<List<DocWindow>> build() async {
    final imported = docsImportEvents.listen((_) => ref.invalidateSelf());
    ref.onDispose(imported.cancel);
    final repo = await ref.watch(docsRepositoryProvider.future);
    return repo.list();
  }

  Future<DocsRepository> _repo() => ref.read(docsRepositoryProvider.future);

  Future<DocWindow?> importNew(String pickedPath, String name) async {
    DocWindow? imported;
    state = const AsyncLoading();
    state = await AsyncValue.guard(() async {
      final repo = await _repo();
      final windows = await repo.list();
      imported = await repo.importFile(sourcePath: pickedPath, name: name);
      windows.add(imported!);
      return windows;
    });
    return imported;
  }

  /// Switches a spreadsheet between the mess menu and table views. Turning it
  /// on parses the file first so a sheet in another format is refused.
  Future<void> setMessMenu(DocWindow doc, bool value) async {
    final repo = await _repo();
    if (value) await loadMessMenu(repo, doc);
    await repo.setMessMenu(doc.id, value);
    ref.invalidateSelf();
  }

  Future<void> rename(String id, String name) async {
    final repo = await _repo();
    await repo.rename(id, name);
    ref.invalidateSelf();
  }

  Future<void> remove(String id) async {
    final repo = await _repo();
    await repo.remove(id);
    ref.invalidateSelf();
  }

  Future<void> saveScrollState(
    String id, {
    required double scale,
    required double offsetX,
    required double offsetY,
    double scrollOffset = 0.0,
  }) async {
    final repo = await _repo();
    await repo.saveScrollState(
      id,
      scale: scale,
      offsetX: offsetX,
      offsetY: offsetY,
      scrollOffset: scrollOffset,
    );
    final windows = state.value;
    if (windows != null) {
      state = AsyncData([
        for (final window in windows)
          if (window.id == id)
            window.copyWith(
              scale: scale,
              offsetX: offsetX,
              offsetY: offsetY,
              scrollOffset: scrollOffset,
            )
          else
            window,
      ]);
    }
  }

  Future<void> touchLastOpened(String id) async {
    final openedAt = DateTime.now().millisecondsSinceEpoch;
    final windows = state.value;
    if (windows != null) {
      state = AsyncData([
        for (final window in windows)
          if (window.id == id)
            window.copyWith(lastOpenedAt: openedAt)
          else
            window,
      ]);
    }
    final repo = await _repo();
    await repo.touchLastOpened(id, openedAt: openedAt);
  }
}
