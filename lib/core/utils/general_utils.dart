import 'dart:developer';
import 'dart:io';

import 'package:background_downloader/background_downloader.dart';
import 'package:dio/dio.dart';
import 'package:intl/intl.dart';
import 'package:path_provider/path_provider.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:pocketbase/pocketbase.dart';
import 'package:vitapmate/core/exceptions.dart';
import 'package:vitapmate/src/api/vtop/vtop_errors.dart';

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

Future<void> downloadFile(String url, String cookie) async {
  Directory? downloadsDir;
  final dio = Dio();
  final response = await dio.head(
    url,
    options: Options(
      responseType: ResponseType.stream,
      followRedirects: true,
      headers: {"Cookie": cookie},
    ),
  );

  final content = response.headers.value(HttpHeaders.contentDisposition);

  if (Platform.isAndroid) {
    if (await Permission.notification.request().isDenied) {
      log("notification permission denied");
    }
    downloadsDir = await getApplicationDocumentsDirectory();
  } else if (Platform.isIOS) {
    downloadsDir = await getApplicationDocumentsDirectory();
  }

  if (downloadsDir == null || !await downloadsDir.exists()) {
    log("Downloads directory not found");
    return;
  }
  FileDownloader().configure(
    androidConfig: (Config.useExternalStorage, Config.always),
  );
  FileDownloader().configureNotification(
    running: TaskNotification('Downloading', 'file: {filename}'),
    complete: TaskNotification('Download finished', 'file: {filename}'),
    tapOpensFile: true,
    progressBar: true,
  );

  String extractFilename(String? header) {
    if (header == null || !header.contains('filename=')) {
      return 'downloaded_file.zip';
    }
    final regex = RegExp(r'filename="?([^"]+)"?');
    final match = regex.firstMatch(header);
    if (match != null) {
      return match.group(1)!.trim();
    }
    return 'downloaded_file.zip';
  }

  final filename = extractFilename(content);

  final task = DownloadTask(
    url: url,
    headers: {"Cookie": cookie},
    retries: 5,
    directory: downloadsDir.path,
    filename: filename,
    allowPause: true,
  );
  final result = await FileDownloader().download(task);

  log("Download started with taskId: $result");
}
