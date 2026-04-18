import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

class JsonFileStorage {
  final String username;

  const JsonFileStorage({required this.username});

  String _normalize(String value) {
    return value.replaceAll(RegExp(r'[^a-zA-Z0-9._-]'), '_');
  }

  Future<File> _fileFor(String key) async {
    final root = await getApplicationSupportDirectory();
    final safeUser = _normalize(username.isEmpty ? 'NO_USERNAME' : username);
    final cacheDir = Directory('${root.path}/cache/$safeUser');
    if (!await cacheDir.exists()) {
      await cacheDir.create(recursive: true);
    }
    final safeKey = _normalize(key);
    return File('${cacheDir.path}/$safeKey.json');
  }

  Future<Map<String, dynamic>?> readJson(String key) async {
    final file = await _fileFor(key);
    if (!await file.exists()) return null;
    final raw = await file.readAsString();
    if (raw.trim().isEmpty) return null;
    final decoded = jsonDecode(raw);
    if (decoded is Map<String, dynamic>) return decoded;
    return null;
  }

  Future<void> writeJson(String key, Map<String, dynamic> data) async {
    final file = await _fileFor(key);
    final temp = File('${file.path}.tmp');
    await temp.writeAsString(jsonEncode(data));
    if (await file.exists()) {
      await file.delete();
    }
    await temp.rename(file.path);
  }

  Future<void> delete(String key) async {
    final file = await _fileFor(key);
    if (await file.exists()) {
      await file.delete();
    }
  }
}
