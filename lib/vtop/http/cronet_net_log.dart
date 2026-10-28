import 'dart:async';
import 'dart:io';

import 'package:cronet_http/cronet_http.dart';
import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';

/// Cronet's net log, for seeing why a request went over HTTP/2 instead of
/// HTTP/3: QUIC lost the race to TCP, failed, or was marked broken. Android
/// only. Open the file at https://netlog-viewer.appspot.com (QUIC and
/// Alt-Svc tabs, or Events filtered by the host).
///
/// Only basic events are logged: no cookies, credentials or bodies, but
/// URLs (with their query strings) are in it.
class CronetNetLog {
  CronetNetLog._();

  /// Whether this platform has a Cronet engine to log.
  static bool get available => Platform.isAndroid;

  /// Whether a log is being written.
  static final recording = ValueNotifier<bool>(false);

  /// Seconds to log from the moment the engine starts, for problems that
  /// begin before the Debug page can be reached. Set on test builds only:
  /// `--dart-define=NETLOG_AT_LAUNCH=90`. The log goes to the app's external
  /// files directory (readable over adb) as `cronet-netlog-launch.json`.
  static const _launchSeconds = int.fromEnvironment('NETLOG_AT_LAUNCH');

  /// Starts the launch log on [engine] when [_launchSeconds] is set.
  static Future<void> recordLaunch(CronetEngine engine) async {
    if (_launchSeconds <= 0) return;
    try {
      final dir = await getExternalStorageDirectory();
      if (dir == null) return;
      engine.startNetLogToFile('${dir.path}/cronet-netlog-launch.json', false);
      Timer(const Duration(seconds: _launchSeconds), engine.stopNetLog);
    } catch (_) {
      // A test aid; the app runs the same without it.
    }
  }

  static Future<File> _file() async =>
      File('${(await getTemporaryDirectory()).path}/cronet-netlog.json');

  /// Starts a new log; it covers every live engine in the app, background
  /// isolates' included.
  static Future<void> start() async {
    final engine = await sharedCronetEngine;
    if (engine == null || recording.value) return;
    final file = await _file();
    if (await file.exists()) await file.delete();
    engine.startNetLogToFile(file.path, false);
    recording.value = true;
  }

  /// Stops the log and returns it, or null when there is none.
  static Future<Uint8List?> stop() async {
    final engine = await sharedCronetEngine;
    if (engine == null || !recording.value) return null;
    engine.stopNetLog();
    recording.value = false;
    final file = await _file();
    return await file.exists() ? file.readAsBytes() : null;
  }
}
