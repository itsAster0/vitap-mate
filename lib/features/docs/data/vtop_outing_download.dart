import 'dart:async';
import 'dart:io';

import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/features/docs/data/doc_models.dart';
import 'package:vitapmate/features/docs/data/docs_repository.dart';

enum VtopOuting {
  weekend('Weekend Outing'),
  general('General Outing');

  const VtopOuting(this.label);
  final String label;
}

VtopOuting? outingForDownload(Iterable<String?> context) {
  for (final value in context) {
    final path = value?.toLowerCase() ?? '';
    if (path.contains('studentweekendouting')) return VtopOuting.weekend;
    if (path.contains('studentgeneralouting')) return VtopOuting.general;
  }
  return null;
}

Future<void> _pendingImport = Future.value();

Future<DocWindow> saveVtopOutingDownloadToDocs({
  required DocsRepository repository,
  required VtopOuting outing,
  required String sourcePath,
  required String filename,
  String? archiveId,
}) async {
  final file = File(sourcePath);
  if (!await file.exists() || await file.length() == 0) {
    throw StateError('The outing download is missing or empty.');
  }
  final cleanName = filename.split(RegExp(r'[/\\]')).last.trim();
  final title =
      '${outing.label} - ${cleanName.isEmpty ? file.uri.pathSegments.last : cleanName}';
  final result = _pendingImport.then(
    (_) => repository.importFile(
      sourcePath: sourcePath,
      name: title,
      id: archiveId,
    ),
  );
  _pendingImport = result.then<void>(
    (_) {},
    onError: (Object _, StackTrace _) {},
  );
  return result;
}

final _watching = <int>{};

Future<void> watchOutingDownload(
  int id, {
  void Function()? onImported,
  void Function()? onFailed,
}) async {
  if (!_watching.add(id)) return;
  try {
    while (true) {
      final copy = await copyCompletedDownload(id);
      if (copy.status == 'pending') {
        await Future<void>.delayed(const Duration(seconds: 2));
        continue;
      }
      if (copy.status == 'failed') {
        await acknowledgeOutingDownload(id);
        onFailed?.call();
        return;
      }
      if (copy.status != 'complete' || copy.path == null) {
        throw StateError('Could not copy the completed download.');
      }
      final jobs = await pendingOutingDownloads();
      final job = jobs.where((item) => item['id'] == id).firstOrNull;
      if (job == null) return;
      final account = job['account'] as String;
      final outing = VtopOuting.values.byName(job['outing'] as String);
      final filename = job['filename'] as String;
      final repository = DocsRepository(JsonFileStorage(username: account));
      await saveVtopOutingDownloadToDocs(
        repository: repository,
        outing: outing,
        sourcePath: copy.path!,
        filename: filename,
        archiveId: 'vtop-outing-$id',
      );
      await acknowledgeOutingDownload(id);
      onImported?.call();
      return;
    }
  } catch (_) {
    onFailed?.call();
    // Keep the pending job so a later launch can retry the local copy.
  } finally {
    _watching.remove(id);
  }
}

Future<void> resumePendingOutingDownloads({void Function()? onImported}) async {
  for (final job in await pendingOutingDownloads()) {
    final id = job['id'];
    if (id is int) {
      unawaited(watchOutingDownload(id, onImported: onImported));
    }
  }
}
