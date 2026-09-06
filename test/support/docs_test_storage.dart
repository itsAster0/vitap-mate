import 'package:vitapmate/core/storage/json_file_storage.dart';

class DocsTestStorage extends JsonFileStorage {
  DocsTestStorage() : super(username: 'docs-test');

  final data = <String, Map<String, dynamic>>{};
  bool failRead = false;
  bool failWrite = false;

  @override
  Future<Map<String, dynamic>?> readJson(String key) async {
    if (failRead) throw StateError('Read failed');
    return data[key];
  }

  @override
  Future<void> writeJson(String key, Map<String, dynamic> value) async {
    if (failWrite) throw StateError('Write failed');
    data[key] = value;
  }

  @override
  Future<String> copyIntoUserDir(
    String subDir,
    String sourcePath, {
    String? fileName,
  }) async => '/test/$subDir/$fileName';
}
