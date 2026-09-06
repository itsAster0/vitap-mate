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

class DownloadReceipt {
  const DownloadReceipt({this.androidId, this.path});
  final int? androidId;
  final String? path;
}

Future<List<Map<String, dynamic>>> pendingOutingDownloads() async {
  if (!Platform.isAndroid) return [];
  final raw = await _downloadManagerChannel.invokeListMethod<dynamic>(
    'pendingOutingDownloads',
  );
  return [
    for (final item in raw ?? const [])
      if (item is Map) Map<String, dynamic>.from(item),
  ];
}

Future<({String status, String? path})> copyCompletedDownload(int id) async {
  final result = await _downloadManagerChannel.invokeMapMethod<String, dynamic>(
    'copyCompletedDownload',
    {'id': id},
  );
  return (
    status: result?['status'] as String? ?? 'failed',
    path: result?['path'] as String?,
  );
}

Future<void> acknowledgeOutingDownload(int id) =>
    _downloadManagerChannel.invokeMethod<void>('ackOutingDownload', {'id': id});
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

Future<DownloadReceipt?> downloadFile(
  String url,
  String cookie, {
  String? contentDisposition,
  String? mimeType,
  String? suggestedFilename,
  String? userAgent,
  String? referer,
  String? archiveOuting,
  String? archiveAccount,
}) async {
  Directory? downloadsDir;

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
      archiveOuting: archiveOuting,
      archiveAccount: archiveAccount,
    );
    if (downloadId != null) {
      return DownloadReceipt(androidId: downloadId);
    }
    downloadsDir = _androidDir;
  } else if (Platform.isIOS) {
    downloadsDir = await getApplicationDocumentsDirectory();
  }

  if (downloadsDir == null || !await downloadsDir.exists()) {
    return null;
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
  return DownloadReceipt(path: await task.filePath());
}

Future<int?> _downloadWithAndroidDownloadManager(
  String url,
  String cookie, {
  String? contentDisposition,
  String? mimeType,
  String? suggestedFilename,
  String? userAgent,
  String? referer,
  String? archiveOuting,
  String? archiveAccount,
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
          'archiveOuting': archiveOuting,
          'archiveAccount': archiveAccount,
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
