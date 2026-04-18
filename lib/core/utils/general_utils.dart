import 'dart:io';
import 'package:background_downloader/background_downloader.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:open_file/open_file.dart';
import 'package:path_provider/path_provider.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:vitapmate/core/utils/app_errors.dart';

final _androidDir = Directory('/storage/emulated/0/Download');
const _downloadManagerChannel = MethodChannel('vitapmate/download_manager');
String formatUnixTimestamp(int timestamp) {
  final date = DateTime.fromMillisecondsSinceEpoch(timestamp * 1000);
  final formatter = DateFormat("MMM dd, yyyy hh:mm a");
  return formatter.format(date).toLowerCase();
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
}) async {
  Directory? downloadsDir;

  if (Platform.isAndroid) {
    await Permission.notification.request();
    final queuedInSystemDownloads = await _downloadWithAndroidDownloadManager(
      url,
      cookie,
      contentDisposition: contentDisposition,
      mimeType: mimeType,
      suggestedFilename: suggestedFilename,
      userAgent: userAgent,
      referer: referer,
    );
    if (queuedInSystemDownloads) {
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
  await FileDownloader().download(task);
}

Future<bool> _downloadWithAndroidDownloadManager(
  String url,
  String cookie, {
  String? contentDisposition,
  String? mimeType,
  String? suggestedFilename,
  String? userAgent,
  String? referer,
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
        });
    return downloadId != null && downloadId > 0;
  } on MissingPluginException {
    return false;
  } on PlatformException {
    return false;
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
