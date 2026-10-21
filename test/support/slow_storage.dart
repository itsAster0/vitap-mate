import 'package:vitapmate/core/storage/json_file_storage.dart';

/// In-memory storage whose reads and writes take a moment, so concurrent
/// saves overlap the way they do on a phone.
class SlowStorage extends JsonFileStorage {
  SlowStorage() : super(username: 'storage-test');

  final data = <String, Map<String, dynamic>>{};

  @override
  Future<Map<String, dynamic>?> readJson(String key) async {
    await Future<void>.delayed(const Duration(milliseconds: 5));
    final value = data[key];
    return value == null ? null : Map<String, dynamic>.from(value);
  }

  @override
  Future<void> writeJson(String key, Map<String, dynamic> value) async {
    await Future<void>.delayed(const Duration(milliseconds: 5));
    data[key] = value;
  }

  @override
  Future<void> delete(String key) async => data.remove(key);
}
