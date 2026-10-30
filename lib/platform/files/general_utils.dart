import 'dart:async';
import 'dart:io';
import 'package:vitapmate/observability/log.dart';
import 'package:background_downloader/background_downloader.dart';
import 'package:flutter/services.dart';
import 'package:open_file/open_file.dart';
import 'package:path_provider/path_provider.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:vitapmate/vtop/app_errors.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:vitapmate/platform/webview/vtop_webview_store.dart';
import 'package:vitapmate/platform/storage/json_file_storage.dart';
import 'package:vitapmate/features/docs/docs_repository.dart';
import 'package:vitapmate/features/docs/download_to_docs.dart';

final _androidDir = Directory('/storage/emulated/0/Download');
const _downloadManagerChannel = MethodChannel('vitapmate/download_manager');

class DocsCopyException implements Exception {
  const DocsCopyException(this.cause);
  final Object cause;
}

String commonErrorMessage(Object e) => appError(e).$2;

void _onNotificationTap(Task task, NotificationType notificationType) async {
  if (notificationType != NotificationType.complete) return;
  if (Platform.isAndroid) return _openAndroidDownloadsFolder();
  final result = await OpenFile.open(await task.filePath());
  if (result.type == ResultType.noAppToOpen) {
    await OpenFile.open(_androidDir.path);
  }
}

void fileDownloaderConfig() {
  FileDownloader().configureNotification(
    running: TaskNotification('Downloading', 'file: {filename}'),
    complete: TaskNotification('Download finished', 'file: {filename}'),
    progressBar: true,
  );

  FileDownloader().registerCallbacks(
    taskNotificationTapCallback: _onNotificationTap,
  );
}

Future<void> downloadFile(
  String url,
  String cookie, {
  String? contentDisposition,
  String? mimeType,
  String? suggestedFilename,
  String? userAgent,
  String? referer,
  bool saveToDocs = false,
}) async {
  Directory? downloadsDir;
  final account = saveToDocs ? vtopWebviewStore.username : null;

  if (Platform.isAndroid) {
    await Permission.notification.request();
    final downloadId = await _downloadWithAndroidDownloadManager(
      url,
      cookie,
      contentDisposition: contentDisposition,
      mimeType: mimeType,
      suggestedFilename: suggestedFilename,
      userAgent: userAgent,
      referer: referer,
      saveToDocs: saveToDocs && account != null,
      docsAccount: account,
    );
    if (downloadId != null) {
      if (saveToDocs && account != null) {
        unawaited(watchDocsDownload(downloadId));
      }
      return;
    }
    downloadsDir = _androidDir;
  } else if (Platform.isIOS) {
    downloadsDir = await getApplicationDocumentsDirectory();
  }

  if (downloadsDir == null || !await downloadsDir.exists()) {
    return;
  }

  final fallbackFilename = _normalizeDownloadFilename(suggestedFilename);
  final task = DownloadTask(
    url: url,
    headers: {"Cookie": cookie},
    retries: 5,
    directory: downloadsDir.path,
    filename: fallbackFilename ?? DownloadTask.suggestedFilename,
    baseDirectory: BaseDirectory.root,
    allowPause: true,
  );
  final result = await loggedNetCall(
    'download ${urlForLog(Uri.tryParse(url))}',
    () => FileDownloader().download(task),
  );
  Log.info(
    'net',
    'download ${urlForLog(Uri.tryParse(url))} ${result.status.name}',
  );
  if (result.status != TaskStatus.complete) {
    throw StateError('Download did not complete: ${result.status.name}');
  }
  if (saveToDocs) {
    if (account == null) {
      Log.info(
        'platform.files',
        'Downloaded file could not be copied to Docs: no active account',
      );
      return;
    }
    final path = await task.filePath();
    try {
      await saveDownloadedFileToDocs(
        repository: DocsRepository(JsonFileStorage(username: account)),
        sourcePath: path,
        filename: path.split('/').last,
      );
    } catch (error) {
      throw DocsCopyException(error);
    }
  }
}

Future<int?> _downloadWithAndroidDownloadManager(
  String url,
  String cookie, {
  String? contentDisposition,
  String? mimeType,
  String? suggestedFilename,
  String? userAgent,
  String? referer,
  bool saveToDocs = false,
  String? docsAccount,
}) async {
  try {
    // Android's download manager fetches it; only the hand-off is seen here.
    final downloadId = await loggedNetCall(
      'download ${urlForLog(Uri.tryParse(url))} handed to Android',
      () => _downloadManagerChannel.invokeMethod<int>('enqueueDownload', {
        'url': url,
        'cookie': cookie,
        'contentDisposition': contentDisposition,
        'mimeType': mimeType,
        'suggestedFilename': _normalizeDownloadFilename(suggestedFilename),
        'userAgent': userAgent,
        'referer': referer,
        'saveToDocs': saveToDocs,
        'docsAccount': docsAccount,
      }),
    );
    Log.info('net', 'download ${urlForLog(Uri.tryParse(url))} is #$downloadId');
    return downloadId != null && downloadId > 0 ? downloadId : null;
  } on MissingPluginException {
    return null;
  } on PlatformException {
    return null;
  }
}

/// VTOP serves some zip files as .bin.
String? _normalizeDownloadFilename(String? filename) =>
    switch (filename?.trim()) {
      null || '' => null,
      final name when name.toLowerCase().endsWith('.bin') =>
        '${name.substring(0, name.length - 4)}.zip',
      final name => name,
    };

Future<void> _openAndroidDownloadsFolder() async {
  try {
    await _downloadManagerChannel.invokeMethod<void>('openDownloadsFolder');
  } on MissingPluginException {
    await OpenFile.open(_androidDir.path);
  } on PlatformException {
    await OpenFile.open(_androidDir.path);
  }
}
