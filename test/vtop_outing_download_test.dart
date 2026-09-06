import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/data/docs_repository.dart';
import 'package:vitapmate/features/docs/data/vtop_outing_download.dart';

class _Storage extends JsonFileStorage {
  _Storage(this.root) : super(username: 'outing-test');

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
  test('outing classification uses the selected menu or download path', () {
    expect(
      outingForDownload(['hostel/StudentWeekendOuting', '/vtop/file?id=1']),
      VtopOuting.weekend,
    );
    expect(
      outingForDownload(['/vtop/hostel/StudentGeneralOuting/print']),
      VtopOuting.general,
    );
    expect(outingForDownload(['/vtop/marks/download']), isNull);
  });

  test('the completed download is copied into Docs', () async {
    final root = await Directory.systemTemp.createTemp('outing-docs-test-');
    addTearDown(() => root.delete(recursive: true));
    final source = File('${root.path}/permit.pdf')
      ..writeAsStringSync('%PDF-1.7 outing');
    final repository = DocsRepository(_Storage(root));

    final doc = await saveVtopOutingDownloadToDocs(
      repository: repository,
      outing: VtopOuting.weekend,
      sourcePath: source.path,
      filename: 'permit.pdf',
      archiveId: 'vtop-outing-42',
    );

    expect(doc.name, 'Weekend Outing - permit.pdf');
    expect(doc.kind, DocKind.pdf);
    expect(
      (await repository.list()).where((item) => item.id == doc.id),
      hasLength(1),
    );
    expect(await source.readAsString(), '%PDF-1.7 outing');
    expect(
      await File('${root.path}/${doc.fileName}').readAsString(),
      '%PDF-1.7 outing',
    );

    final repeated = await saveVtopOutingDownloadToDocs(
      repository: repository,
      outing: VtopOuting.weekend,
      sourcePath: source.path,
      filename: 'permit.pdf',
      archiveId: 'vtop-outing-42',
    );
    expect(repeated.id, doc.id);
    expect(
      (await repository.list()).where((item) => item.hasFile),
      hasLength(1),
    );
  });

  test(
    'other downloaded types remain available as generic Docs files',
    () async {
      final root = await Directory.systemTemp.createTemp('outing-docs-test-');
      addTearDown(() => root.delete(recursive: true));
      final source = File('${root.path}/forms.zip')
        ..writeAsStringSync('archive');
      final repository = DocsRepository(_Storage(root));

      final doc = await saveVtopOutingDownloadToDocs(
        repository: repository,
        outing: VtopOuting.general,
        sourcePath: source.path,
        filename: 'forms.zip',
      );

      expect(doc.kind, DocKind.file);
      expect(doc.hasFile, isTrue);
      expect(await source.readAsString(), 'archive');
    },
  );

  test('simultaneous local copies both remain in Docs', () async {
    final root = await Directory.systemTemp.createTemp('outing-docs-test-');
    addTearDown(() => root.delete(recursive: true));
    final one = File('${root.path}/one.pdf')..writeAsStringSync('one');
    final two = File('${root.path}/two.pdf')..writeAsStringSync('two');
    final repository = DocsRepository(_Storage(root));

    await Future.wait([
      saveVtopOutingDownloadToDocs(
        repository: repository,
        outing: VtopOuting.weekend,
        sourcePath: one.path,
        filename: 'one.pdf',
      ),
      saveVtopOutingDownloadToDocs(
        repository: repository,
        outing: VtopOuting.general,
        sourcePath: two.path,
        filename: 'two.pdf',
      ),
    ]);
    final docs = (await repository.list()).where((doc) => doc.hasFile).toList();
    expect(docs, hasLength(2));
    expect(docs.map((doc) => doc.fileName).toSet(), hasLength(2));
  });

  test('missing or empty source is never added to Docs', () async {
    final root = await Directory.systemTemp.createTemp('outing-docs-test-');
    addTearDown(() => root.delete(recursive: true));
    final repository = DocsRepository(_Storage(root));

    await expectLater(
      saveVtopOutingDownloadToDocs(
        repository: repository,
        outing: VtopOuting.weekend,
        sourcePath: '${root.path}/missing.pdf',
        filename: 'missing.pdf',
      ),
      throwsStateError,
    );
    expect((await repository.list()).where((doc) => doc.hasFile), isEmpty);
  });
}
