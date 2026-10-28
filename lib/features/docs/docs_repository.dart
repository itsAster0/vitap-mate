import 'dart:developer';
import 'dart:io';

import 'package:vitapmate/platform/storage/json_file_storage.dart';
import 'package:vitapmate/features/docs/logic/doc_models.dart';

class DocsRepository {
  static const _registryKey = 'docs_registry';
  static const _fileSubDir = 'docs';

  final JsonFileStorage storage;
  int _nextImportId = 0;
  DocsRepository(this.storage);

  Future<List<DocWindow>> list() async {
    final data = await storage.readJson(_registryKey);
    final raw = data?['windows'] as List<dynamic>?;
    if (raw == null) return [];
    return raw
        .whereType<Map<String, dynamic>>()
        .map(DocWindow.fromJson)
        // Older versions kept an empty "Mess Menu" slot; a filled one stays
        // as an ordinary document.
        .where((w) => !(w.isPreset && !w.hasFile))
        .toList();
  }

  Future<void> _writeAll(List<DocWindow> windows) => storage.writeJson(
    _registryKey,
    {'windows': windows.map((w) => w.toJson()).toList()},
  );

  Future<DocWindow> importFile({
    required String sourcePath,
    String? name,
    String? id,
  }) async {
    final windows = await list();
    if (windows.where((w) => id != null && w.id == id).firstOrNull
        case final existing?) {
      return existing;
    }
    final ext = sourcePath.split('.').last.toLowerCase();
    final importId =
        '${DateTime.now().microsecondsSinceEpoch}-${_nextImportId++}';
    final storedName = 'doc-$importId.$ext';
    await storage.copyIntoUserDir(
      _fileSubDir,
      sourcePath,
      fileName: storedName,
    );
    final cleanName = (name ?? sourcePath.split('/').last).trim().isEmpty
        ? storedName
        : (name ?? storedName).trim();
    final doc = DocWindow(
      id: id ?? 'doc-$importId',
      name: cleanName,
      kind: docKindFromExtension(sourcePath),
      fileName: storedName,
      addedAt: DateTime.now().millisecondsSinceEpoch,
    );
    windows.add(doc);
    await _writeAll(windows);
    return doc;
  }

  /// Replaces the document [id] with [change] of it, if it is there.
  Future<void> _update(String id, DocWindow Function(DocWindow) change) async {
    final windows = await list();
    final idx = windows.indexWhere((w) => w.id == id);
    if (idx == -1) return;
    windows[idx] = change(windows[idx]);
    await _writeAll(windows);
  }

  Future<void> setMessMenu(String id, bool value) =>
      _update(id, (w) => w.copyWith(asMessMenu: value));

  Future<void> rename(String id, String name) =>
      _update(id, (w) => w.copyWith(name: name.trim()));

  Future<void> remove(String id) async {
    final windows = await list();
    final target = windows.where((w) => w.id == id).firstOrNull;
    if (target == null) return;
    if (target.fileName != null) {
      try {
        final dir = await storage.userDir(_fileSubDir, create: false);
        await storage.deleteUserFile('${dir.path}/${target.fileName}');
      } catch (e) {
        log('docs: failed deleting file: $e');
      }
    }
    windows.removeWhere((w) => w.id == id);
    await _writeAll(windows);
  }

  Future<void> saveScrollState(
    String id, {
    required double scale,
    required double offsetX,
    required double offsetY,
    double scrollOffset = 0.0,
  }) => _update(
    id,
    (w) => w.copyWith(
      scale: scale,
      offsetX: offsetX,
      offsetY: offsetY,
      scrollOffset: scrollOffset,
    ),
  );

  Future<void> touchLastOpened(String id, {required int openedAt}) =>
      _update(id, (w) => w.copyWith(lastOpenedAt: openedAt));

  Future<String?> storedFilePathOf(DocWindow window) async {
    if (window.fileName == null) return null;
    final dir = await storage.userDir(_fileSubDir);
    final file = File('${dir.path}/${window.fileName}');
    if (!await file.exists()) return null;
    return file.path;
  }
}
