import 'dart:io';

import 'package:path_provider/path_provider.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/src/api/native_logs.dart';
import 'package:vitapmate/src/frb_generated.dart';

bool _started = false;

/// Starts the Rust library and this isolate's log (the app, background sync
/// and push messages each run in their own isolate). Safe to call again.
Future<void> startRustRuntime() async {
  if (_started) return;
  _started = true;
  await RustLib.init();
  final dir = Directory(
    '${(await getApplicationSupportDirectory()).path}/logs',
  );
  await Log.open(dir);
  await nativeLogsOpen(dir: dir.path);
}
