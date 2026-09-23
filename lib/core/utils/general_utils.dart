import 'dart:async';
import 'dart:developer';
import 'dart:io';
import 'package:background_downloader/background_downloader.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:open_file/open_file.dart';
import 'package:path_provider/path_provider.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:vitapmate/core/utils/app_errors.dart';
import 'package:vitapmate/core/utils/vtop_webview_store.dart';
import 'package:vitapmate/core/storage/json_file_storage.dart';
import 'package:vitapmate/features/docs/data/docs_repository.dart';
import 'package:vitapmate/features/docs/data/download_to_docs.dart';

final _androidDir = Directory('/storage/emulated/0/Download');
const _downloadManagerChannel = MethodChannel('vitapmate/download_manager');

class DocsCopyException implements Exception {
  const DocsCopyException(this.cause);
  final Object cause;
}

String formatUnixTimestamp(int timestamp) {
  final date = DateTime.fromMillisecondsSinceEpoch(timestamp * 1000);
  final formatter = DateFormat("MMM dd, yyyy hh:mm a");
  final formatted = formatter.format(date);
  return formatted.replaceFirstMapped(
    RegExp(r'^[A-Za-z]{3}'),
    (match) => match.group(0)!.toUpperCase(),
  );
}

String commonErrorMessage(Object e) {
  final (type, message) = appError(e);
  return message;
}

void myNotificationTapCallback(
  Task task,
  NotificationType notificationType,
) async {
  if (notificationType == NotificationType.complete) {
    if (Platform.isAndroid) {
      await _openAndroidDownloadsFolder();
      return;
    }

    final path = await task.filePath();
    final result = await OpenFile.open(path);
    if (result.type == ResultType.noAppToOpen) {
      await OpenFile.open(_androidDir.path);
    }
  }
}

void fileDownloaderConfig() {
  FileDownloader().configureNotification(
    running: TaskNotification('Downloading', 'file: {filename}'),
    complete: TaskNotification('Download finished', 'file: {filename}'),

    progressBar: true,
  );

  FileDownloader().registerCallbacks(
    taskNotificationTapCallback: myNotificationTapCallback,
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
  final result = await FileDownloader().download(task);
  if (result.status != TaskStatus.complete) {
    throw StateError('Download did not complete: ${result.status.name}');
  }
  if (saveToDocs) {
    if (account == null) {
      log('Downloaded file could not be copied to Docs: no active account');
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
    final downloadId = await _downloadManagerChannel
        .invokeMethod<int>('enqueueDownload', {
          'url': url,
          'cookie': cookie,
          'contentDisposition': contentDisposition,
          'mimeType': mimeType,
          'suggestedFilename': _normalizeDownloadFilename(suggestedFilename),
          'userAgent': userAgent,
          'referer': referer,
          'saveToDocs': saveToDocs,
          'docsAccount': docsAccount,
        });
    return downloadId != null && downloadId > 0 ? downloadId : null;
  } on MissingPluginException {
    return null;
  } on PlatformException {
    return null;
  }
}

String? _normalizeDownloadFilename(String? filename) {
  final trimmed = filename?.trim();
  if (trimmed == null || trimmed.isEmpty) {
    return null;
  }

  if (trimmed.toLowerCase().endsWith('.bin')) {
    return '${trimmed.substring(0, trimmed.length - 4)}.zip';
  }

  return trimmed;
}

Future<void> _openAndroidDownloadsFolder() async {
  try {
    await _downloadManagerChannel.invokeMethod<void>('openDownloadsFolder');
  } on MissingPluginException {
    await OpenFile.open(_androidDir.path);
  } on PlatformException {
    await OpenFile.open(_androidDir.path);
  }
}
