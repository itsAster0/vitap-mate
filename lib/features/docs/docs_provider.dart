import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/foundation.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/platform/storage/json_file_storage_provider.dart';
import 'package:vitapmate/features/docs/logic/doc_models.dart';
import 'package:vitapmate/features/docs/docs_repository.dart';
import 'package:vitapmate/features/docs/download_to_docs.dart';
import 'package:vitapmate/features/docs/logic/spreadsheet_tables.dart';
import 'package:vitapmate/features/docs/logic/mess_menu.dart';

Future<String?> pickDocPath() async {
  final file = await FilePicker.pickFile(
    type: FileType.custom,
    allowedExtensions: [for (final e in docExtensions.values) ...e],
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
    await (await _repo()).rename(id, name);
    ref.invalidateSelf();
  }

  Future<void> remove(String id) async {
    await (await _repo()).remove(id);
    ref.invalidateSelf();
  }

  /// Shows [change] on [id] at once, without reloading the list.
  void _patch(String id, DocWindow Function(DocWindow) change) {
    if (state.value case final windows?) {
      state = AsyncData([for (final w in windows) w.id == id ? change(w) : w]);
    }
  }

  Future<void> saveScrollState(
    String id, {
    required double scale,
    required double offsetX,
    required double offsetY,
    double scrollOffset = 0.0,
  }) async {
    await (await _repo()).saveScrollState(
      id,
      scale: scale,
      offsetX: offsetX,
      offsetY: offsetY,
      scrollOffset: scrollOffset,
    );
    _patch(
      id,
      (w) => w.copyWith(
        scale: scale,
        offsetX: offsetX,
        offsetY: offsetY,
        scrollOffset: scrollOffset,
      ),
    );
  }

  Future<void> touchLastOpened(String id) async {
    final openedAt = DateTime.now().millisecondsSinceEpoch;
    _patch(id, (w) => w.copyWith(lastOpenedAt: openedAt));
    await (await _repo()).touchLastOpened(id, openedAt: openedAt);
  }
}
