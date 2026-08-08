import 'dart:developer';
import 'dart:io';

import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';

class DocsRepository {
  static const _registryKey = 'docs_registry';
  static const _fileSubDir = 'docs';
  static const _messPresetId = 'mess-preset';

  final JsonFileStorage storage;
  DocsRepository(this.storage);

  Future<List<DocWindow>> list() async {
    final data = await storage.readJson(_registryKey);
    final raw = data?['windows'] as List<dynamic>?;
    if (raw == null) return [_seedMessPreset()];
    final windows = raw
        .whereType<Map<String, dynamic>>()
        .map(DocWindow.fromJson)
        .toList();
    if (!windows.any((w) => w.id == _messPresetId)) {
      windows.insert(0, _seedMessPreset());
    }
    return windows;
  }

  DocWindow _seedMessPreset() => const DocWindow(
    id: _messPresetId,
    name: 'Mess Menu',
    kind: DocKind.none,
    isPreset: true,
    addedAt: 0,
  );

  Future<void> _writeAll(List<DocWindow> windows) async {
    await storage.writeJson(_registryKey, {
      'windows': windows.map((w) => w.toJson()).toList(),
    });
  }

  Future<DocWindow> importFile({
    required String sourcePath,
    String? name,
  }) async {
    final ext = sourcePath.split('.').last.toLowerCase();
    final storedName = 'doc-${DateTime.now().millisecondsSinceEpoch}.$ext';
    await storage.copyIntoUserDir(
      _fileSubDir,
      sourcePath,
      fileName: storedName,
    );
    final cleanName = (name ?? sourcePath.split('/').last).trim().isEmpty
        ? storedName
        : (name ?? storedName).trim();
    final windows = await list();
    final doc = DocWindow(
      id: 'doc-${DateTime.now().microsecondsSinceEpoch}',
      name: cleanName,
      kind: docKindFromExtension(sourcePath),
      fileName: storedName,
      addedAt: DateTime.now().millisecondsSinceEpoch,
      lastOpenedAt: DateTime.now().millisecondsSinceEpoch,
    );
    windows.add(doc);
    await _writeAll(windows);
    return doc;
  }

  Future<DocWindow?> fillPreset(String presetId, String pickedPath) async {
    final windows = await list();
    final idx = windows.indexWhere((w) => w.id == presetId && !w.hasFile);
    if (idx == -1) return null;
    final ext = pickedPath.split('.').last.toLowerCase();
    final storedName =
        '${windows[idx].id}-${DateTime.now().millisecondsSinceEpoch}.$ext';
    final storedPath = await storage.copyIntoUserDir(
      _fileSubDir,
      pickedPath,
      fileName: storedName,
    );
    final updated = windows[idx].copyWith(
      kind: docKindFromExtension(storedPath),
      fileName: storedName,
      lastOpenedAt: DateTime.now().millisecondsSinceEpoch,
    );
    windows[idx] = updated;
    await _writeAll(windows);
    return updated;
  }

  Future<void> rename(String id, String name) async {
    final windows = await list();
    final idx = windows.indexWhere((w) => w.id == id);
    if (idx == -1) return;
    windows[idx] = windows[idx].copyWith(name: name.trim());
    await _writeAll(windows);
  }

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
  }) async {
    final windows = await list();
    final idx = windows.indexWhere((w) => w.id == id);
    if (idx == -1) return;
    final w = windows[idx];
    windows[idx] = DocWindow(
      id: w.id,
      name: w.name,
      kind: w.kind,
      fileName: w.fileName,
      isPreset: w.isPreset,
      addedAt: w.addedAt,
      lastOpenedAt: w.lastOpenedAt,
      scale: scale,
      offsetX: offsetX,
      offsetY: offsetY,
      scrollOffset: scrollOffset,
    );
    await _writeAll(windows);
  }

  Future<void> touchLastOpened(String id, {required int openedAt}) async {
    final windows = await list();
    final idx = windows.indexWhere((w) => w.id == id);
    if (idx == -1) return;
    windows[idx] = windows[idx].copyWith(lastOpenedAt: openedAt);
    await _writeAll(windows);
  }

  Future<String?> storedFilePathOf(DocWindow window) async {
    if (window.fileName == null) return null;
    final dir = await storage.userDir(_fileSubDir);
    final file = File('${dir.path}/${window.fileName}');
    if (!await file.exists()) return null;
    return file.path;
  }
}
