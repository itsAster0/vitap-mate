import 'dart:async';
import 'dart:developer';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:path_provider/path_provider.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/data/docs_repository.dart';

const _downloadChannel = MethodChannel('vitapmate/download_manager');
final _docsImports = StreamController<void>.broadcast();
Stream<void> get docsImportEvents => _docsImports.stream;

Future<void> _pendingImport = Future.value();

Future<DocWindow> saveDownloadedFileToDocs({
  required DocsRepository repository,
  required String sourcePath,
  required String filename,
  String? archiveId,
}) async {
  final file = File(sourcePath);
  if (!await file.exists() || await file.length() == 0) {
    throw StateError('The downloaded file is missing or empty.');
  }
  final cleanName = filename.split(RegExp(r'[/\\]')).last.trim();
  final name = cleanName.isEmpty ? file.uri.pathSegments.last : cleanName;
  final result = _pendingImport.then(
    (_) => repository.importFile(
      sourcePath: sourcePath,
      name: name,
      id: archiveId,
    ),
  );
  _pendingImport = result.then<void>(
    (_) {},
    onError: (Object _, StackTrace _) {},
  );
  final document = await result;
  _docsImports.add(null);
  return document;
}

/// The document [docId] from Docs, or [download]ed (bytes and the name to
/// file them under) and saved the first time. Keeps app-fetched files (outing passes,
/// course material) available offline without fetching them twice.
Future<DocWindow> keepInDocs({
  required DocsRepository repository,
  required String docId,
  required Future<({List<int> bytes, String fileName})> Function() download,
}) async {
  for (final doc in await repository.list()) {
    if (doc.id == docId && await repository.storedFilePathOf(doc) != null) {
      return doc;
    }
  }
  final downloaded = await download();
  final fileName = downloaded.fileName;
  final safeId = docId.replaceAll(RegExp(r'[^A-Za-z0-9_-]'), '_');
  final extension = fileName.contains('.')
      ? '.${fileName.split('.').last}'
      : '';
  final file = File(
    '${(await getTemporaryDirectory()).path}/$safeId$extension',
  );
  await file.writeAsBytes(downloaded.bytes, flush: true);
  try {
    return await saveDownloadedFileToDocs(
      repository: repository,
      sourcePath: file.path,
      filename: fileName,
      archiveId: docId,
    );
  } finally {
    if (await file.exists()) await file.delete();
  }
}

final _watching = <int>{};

Future<void> watchDocsDownload(int id) async {
  if (!_watching.add(id)) return;
  try {
    while (true) {
      final copy = await _copyCompletedDownload(id);
      if (copy.status == 'pending') {
        await Future<void>.delayed(const Duration(seconds: 2));
        continue;
      }
      if (copy.status == 'failed') {
        await _acknowledgeDownload(id);
        return;
      }
      if (copy.status != 'complete' || copy.path == null) {
        throw StateError('Could not copy the completed download.');
      }
      final jobs = await _pendingDocsDownloads();
      final job = jobs.where((item) => item['id'] == id).firstOrNull;
      if (job == null) return;
      final repository = DocsRepository(
        JsonFileStorage(username: job['account'] as String),
      );
      await saveDownloadedFileToDocs(
        repository: repository,
        sourcePath: copy.path!,
        filename: job['filename'] as String,
        archiveId: 'download-$id',
      );
      await _acknowledgeDownload(id);
      return;
    }
  } catch (error, stackTrace) {
    log(
      'Docs copy failed; will retry on next launch',
      error: error,
      stackTrace: stackTrace,
    );
  } finally {
    _watching.remove(id);
  }
}

Future<void> resumePendingDocsDownloads() async {
  for (final job in await _pendingDocsDownloads()) {
    final id = job['id'];
    if (id is int) unawaited(watchDocsDownload(id));
  }
}

Future<List<Map<String, dynamic>>> _pendingDocsDownloads() async {
  if (!Platform.isAndroid) return [];
  final raw = await _downloadChannel.invokeListMethod<dynamic>(
    'pendingDocsDownloads',
  );
  return [
    for (final item in raw ?? const [])
      if (item is Map) Map<String, dynamic>.from(item),
  ];
}

Future<({String status, String? path})> _copyCompletedDownload(int id) async {
  final result = await _downloadChannel.invokeMapMethod<String, dynamic>(
    'copyCompletedDownload',
    {'id': id},
  );
  return (
    status: result?['status'] as String? ?? 'failed',
    path: result?['path'] as String?,
  );
}

Future<void> _acknowledgeDownload(int id) =>
    _downloadChannel.invokeMethod<void>('ackDocsDownload', {'id': id});
