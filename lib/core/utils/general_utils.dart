import 'dart:developer';
import 'dart:io';
import 'package:background_downloader/background_downloader.dart';
import 'package:intl/intl.dart';
import 'package:open_file/open_file.dart';
import 'package:path_provider/path_provider.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

final _androidDir = Directory('/storage/emulated/0/Download');
String formatUnixTimestamp(int timestamp) {
  final date = DateTime.fromMillisecondsSinceEpoch(timestamp * 1000);
  final formatter = DateFormat("MMM dd, yyyy hh:mm a");
  return formatter.format(date).toLowerCase();
}

String commonErrorMessage(Object e) {
  if (e == VtopError.invalidCredentials()) {
    return 'It looks like you changed your VTOP password. Please update it.';
  } else if (e == VtopError.networkError() || e is ClientException) {
    return "You're offline. Please check your connection and try again by refreshing.";
  } else if (e is FeatureDisabledException) {
    return 'This feature is currently disabled. Please try again in a while';
  } else {
    return "Something went wrong. Please try reopening the app.";
  }
}

void myNotificationTapCallback(
  Task task,
  NotificationType notificationType,
) async {
  if (notificationType == NotificationType.complete) {
    var path = await task.filePath();
    var k = await OpenFile.open(path);
    if (k.type == ResultType.noAppToOpen) {
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

Future<void> downloadFile(String url, String cookie) async {
  Directory? downloadsDir;

  if (Platform.isAndroid) {
    if (await Permission.notification.request().isDenied) {
      log("notification permission denied");
    }
    downloadsDir = _androidDir;
  } else if (Platform.isIOS) {
    downloadsDir = await getApplicationDocumentsDirectory();
  }

  if (downloadsDir == null || !await downloadsDir.exists()) {
    log("Downloads directory not found");
    return;
  }

  final task = DownloadTask(
    url: url,
    headers: {"Cookie": cookie},
    retries: 5,

    directory: downloadsDir.path,
    filename: DownloadTask.suggestedFilename,
    baseDirectory: BaseDirectory.root,
    allowPause: true,
  );
  final result = await FileDownloader().download(task);
  log("Download started with taskId: $result");
}
