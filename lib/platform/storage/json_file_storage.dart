import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

class JsonFileStorage {
  final String username;
  late final Future<Directory> _storageDir = _initStorageDir();

  /// Where the per-user folders go; the app support directory by default.
  final Future<Directory> Function() _root;

  JsonFileStorage({required this.username, Future<Directory> Function()? root})
    : _root = root ?? getApplicationSupportDirectory;

  String _normalize(String value) {
    return value.replaceAll(RegExp(r'[^a-zA-Z0-9._-]'), '_');
  }

  Future<Directory> _initStorageDir() async {
    final storageDir = await _getUserDir('data');
    final legacyCacheDir = await _getUserDir('cache', create: false);
    await _migrateLegacyCacheDir(from: legacyCacheDir, to: storageDir);
    return storageDir;
  }

  Future<Directory> _getUserDir(String name, {bool create = true}) async {
    final root = await _root();
    final safeUser = _normalize(username.isEmpty ? 'NO_USERNAME' : username);
    final dir = Directory('${root.path}/$name/$safeUser');
    if (create && !await dir.exists()) {
      await dir.create(recursive: true);
    }
    return dir;
  }

  Future<void> _migrateLegacyCacheDir({
    required Directory from,
    required Directory to,
  }) async {
    if (!await from.exists()) return;

    await for (final entity in from.list()) {
      if (entity is! File || !entity.path.endsWith('.json')) continue;

      final fileName = entity.uri.pathSegments.last;
      final target = File('${to.path}/$fileName');
      if (!await target.exists()) {
        await entity.rename(target.path);
      } else {
        await entity.delete();
      }
    }
  }

  Future<File> _fileFor(String key) async {
    final storageDir = await _storageDir;
    final safeKey = _normalize(key);
    return File('${storageDir.path}/$safeKey.json');
  }

  Future<Map<String, dynamic>?> readJson(String key) async {
    final file = await _fileFor(key);
    if (!await file.exists()) return null;
    final raw = await file.readAsString();
    if (raw.trim().isEmpty) return null;
    final Object? decoded;
    try {
      decoded = jsonDecode(raw);
    } on FormatException {
      // A file cut short (from before writes were atomic) is a cache miss.
      return null;
    }
    if (decoded is Map<String, dynamic>) return decoded;
    return null;
  }

  /// Writes beside the file, then renames over it, so a write cut short
  /// (the app killed mid-sync) leaves the old file instead of half a new one.
  Future<void> writeJson(String key, Map<String, dynamic> data) async {
    final file = await _fileFor(key);
    final temp = File('${file.path}.tmp');
    await temp.writeAsString(jsonEncode(data), flush: true);
    await temp.rename(file.path);
  }

  Future<void> delete(String key) async {
    final file = await _fileFor(key);
    if (await file.exists()) {
      await file.delete();
    }
  }

  Future<Directory> userDir(String name, {bool create = true}) {
    return _getUserDir(name, create: create);
  }

  Future<String> copyIntoUserDir(
    String subDir,
    String sourcePath, {
    String? fileName,
  }) async {
    final dir = await _getUserDir(subDir);
    final safeName = _normalize(fileName ?? sourcePath.split('/').last);
    final target = File('${dir.path}/$safeName');
    await File(sourcePath).copy(target.path);
    return target.path;
  }

  Future<void> deleteUserFile(String absoluteOrRelativePath) async {
    final file = File(absoluteOrRelativePath);
    if (await file.exists()) {
      await file.delete();
    }
  }
}
