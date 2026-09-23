import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/data/docs_repository.dart';
import 'package:vitapmate/features/docs/data/download_to_docs.dart';

class _Storage extends JsonFileStorage {
  _Storage(this.root) : super(username: 'docs-test');

  final Directory root;
  final data = <String, Map<String, dynamic>>{};

  @override
  Future<Map<String, dynamic>?> readJson(String key) async => data[key];

  @override
  Future<void> writeJson(String key, Map<String, dynamic> value) async {
    data[key] = value;
  }

  @override
  Future<String> copyIntoUserDir(
    String subDir,
    String sourcePath, {
    String? fileName,
  }) async {
    final destination = File('${root.path}/$fileName');
    await File(sourcePath).copy(destination.path);
    return destination.path;
  }
}

void main() {
  test('completed download is copied with its filename', () async {
    final root = await Directory.systemTemp.createTemp('docs-download-test-');
    addTearDown(() => root.delete(recursive: true));
    final source = File('${root.path}/permit.pdf')
      ..writeAsStringSync('%PDF-1.7 outing');
    final repository = DocsRepository(_Storage(root));

    final doc = await saveDownloadedFileToDocs(
      repository: repository,
      sourcePath: source.path,
      filename: 'permit.pdf',
      archiveId: 'download-42',
    );

    expect(doc.name, 'permit.pdf');
    expect(doc.kind, DocKind.pdf);
    expect(await source.readAsString(), '%PDF-1.7 outing');
    expect(
      await File('${root.path}/${doc.fileName}').readAsString(),
      '%PDF-1.7 outing',
    );

    final repeated = await saveDownloadedFileToDocs(
      repository: repository,
      sourcePath: source.path,
      filename: 'permit.pdf',
      archiveId: 'download-42',
    );
    expect(repeated.id, doc.id);
    expect(
      (await repository.list()).where((item) => item.hasFile),
      hasLength(1),
    );
  });

  test('other file types are saved under their own names', () async {
    final root = await Directory.systemTemp.createTemp('docs-download-test-');
    addTearDown(() => root.delete(recursive: true));
    final source = File('${root.path}/forms.zip')..writeAsStringSync('archive');
    final repository = DocsRepository(_Storage(root));

    final doc = await saveDownloadedFileToDocs(
      repository: repository,
      sourcePath: source.path,
      filename: 'forms.zip',
    );

    expect(doc.name, 'forms.zip');
    expect(doc.kind, DocKind.file);
    expect(doc.hasFile, isTrue);
    expect(await source.readAsString(), 'archive');
  });

  test('simultaneous copies both remain in Docs', () async {
    final root = await Directory.systemTemp.createTemp('docs-download-test-');
    addTearDown(() => root.delete(recursive: true));
    final one = File('${root.path}/one.pdf')..writeAsStringSync('one');
    final two = File('${root.path}/two.pdf')..writeAsStringSync('two');
    final repository = DocsRepository(_Storage(root));

    await Future.wait([
      saveDownloadedFileToDocs(
        repository: repository,
        sourcePath: one.path,
        filename: 'one.pdf',
      ),
      saveDownloadedFileToDocs(
        repository: repository,
        sourcePath: two.path,
        filename: 'two.pdf',
      ),
    ]);
    final docs = (await repository.list()).where((doc) => doc.hasFile).toList();
    expect(docs, hasLength(2));
    expect(docs.map((doc) => doc.fileName).toSet(), hasLength(2));
  });

  test('missing file is not added to Docs', () async {
    final root = await Directory.systemTemp.createTemp('docs-download-test-');
    addTearDown(() => root.delete(recursive: true));
    final repository = DocsRepository(_Storage(root));

    await expectLater(
      saveDownloadedFileToDocs(
        repository: repository,
        sourcePath: '${root.path}/missing.pdf',
        filename: 'missing.pdf',
      ),
      throwsStateError,
    );
    expect((await repository.list()).where((doc) => doc.hasFile), isEmpty);
  });
}
