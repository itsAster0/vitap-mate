/// The app's one log: JSON lines in `logs/app.log` in the app's cache, rolled
/// over to `app.log.1` past [Log.maxBytes], next to Rust's `logs/rust.log`
/// (same format, see `rust/src/api/native_logs.rs`). Every line carries the
/// [flow] it belongs to and the student's [Log.regno]; anything that looks
/// like a password, OTP, cookie or token is blanked first. Nothing leaves
/// the phone unless the student copies or saves it.
library;

import 'dart:async';
import 'dart:convert';
import 'dart:developer' as developer;
import 'dart:io';
import 'dart:math';

import 'package:flutter/foundation.dart';

enum LogLevel {
  debug(500),
  info(800),
  warn(900),
  error(1000);

  const LogLevel(this.value);

  /// The level for `dart:developer`.
  final int value;
}

/// One line read back from the log files.
typedef LogLine = ({
  DateTime time,
  String level,
  String source,
  String message,
  String? flow,
  String? regno,

  /// `step`, `ev` (start, done, failed) and `ms` on a [flow] step's lines;
  /// `stack` on errors.
  Map<String, Object?> data,
});

abstract final class Log {
  static const maxBytes = 1024 * 1024;
  static const _files = ['app.log.1', 'app.log', 'rust.log.1', 'rust.log'];

  static Directory? _dir;
  static RandomAccessFile? _file;

  /// Whether debug lines are written too. Set in Developer → Logs; kept as a
  /// file beside the log so every isolate sees it.
  static bool verbose = false;

  /// The signed-in student's registration number, once known.
  static String? regno;

  /// Opens the log in [dir]. Lines written before this only reach the
  /// debugger. Each isolate opens it for itself.
  static Future<void> open(Directory dir) async {
    await dir.create(recursive: true);
    _dir = dir;
    _file = await File('${dir.path}/app.log').open(mode: FileMode.append);
    verbose = File('${dir.path}/verbose').existsSync();
  }

  static Future<void> setVerbose(bool on) async {
    verbose = on;
    final marker = File('${_dir?.path}/verbose');
    if (on) {
      await marker.create();
    } else if (marker.existsSync()) {
      await marker.delete();
    }
  }

  /// Logs uncaught errors (Flutter's and the isolate's) with their stacks.
  static void catchUncaught() {
    final flutter = FlutterError.onError;
    FlutterError.onError = (details) {
      error('flutter', details.exceptionAsString(), details.stack);
      flutter?.call(details);
    };
    final platform = PlatformDispatcher.instance.onError;
    PlatformDispatcher.instance.onError = (exception, stack) {
      error('uncaught', '$exception', stack);
      return platform?.call(exception, stack) ?? false;
    };
  }

  static void debug(String source, String message) =>
      write(.debug, source, message);
  static void info(String source, String message) =>
      write(.info, source, message);
  static void warn(String source, String message) =>
      write(.warn, source, message);
  static void error(String source, String message, [StackTrace? stack]) =>
      write(.error, source, message, {'stack': ?stack?.toString()});

  static void write(
    LogLevel level,
    String source,
    String message, [
    Map<String, Object?> data = const {},
  ]) {
    if (level == .debug && !verbose) return;
    message = scrub(message);
    developer.log(message, name: source, level: level.value);
    final line = jsonEncode({
      't': DateTime.now().millisecondsSinceEpoch,
      'l': level.name,
      'src': source,
      'msg': message,
      'flow': ?currentFlow,
      'regno': ?regno,
      if (data.isNotEmpty) 'd': data,
    });
    // Logging never breaks the app: a full disk only loses lines.
    try {
      if (_file case final file? when file.lengthSync() > maxBytes) _roll();
      _file?.writeStringSync('$line\n');
    } on FileSystemException {
      return;
    }
  }

  static void _roll() {
    final dir = _dir!;
    _file!.closeSync();
    File('${dir.path}/app.log').renameSync('${dir.path}/app.log.1');
    _file = File('${dir.path}/app.log').openSync(mode: FileMode.append);
  }

  /// The size of every log file, in bytes.
  static int get size => [
    for (final name in _files)
      if (File('${_dir?.path}/$name') case final file when file.existsSync())
        file.lengthSync(),
  ].fold(0, (sum, bytes) => sum + bytes);

  /// Every line on the phone, Dart's and Rust's, oldest first.
  static Future<List<LogLine>> read() async {
    final dir = _dir;
    if (dir == null) return const [];
    final lines = [
      for (final name in _files)
        if (File('${dir.path}/$name') case final file when file.existsSync())
          for (final raw in await file.readAsLines()) ?_parse(raw),
    ].indexed.toList();
    // Lines of the same millisecond keep their order in the file.
    lines.sort(
      (a, b) => switch (a.$2.time.compareTo(b.$2.time)) {
        0 => a.$1.compareTo(b.$1),
        final order => order,
      },
    );
    return [for (final (_, line) in lines) line];
  }

  static LogLine? _parse(String raw) {
    try {
      if (jsonDecode(raw)
          case {
                't': final int t,
                'l': final String level,
                'src': final String source,
                'msg': final String message,
              } &&
              final Map json) {
        return (
          time: DateTime.fromMillisecondsSinceEpoch(t),
          level: level,
          source: source,
          message: scrub(message),
          flow: json['flow'] as String?,
          regno: json['regno'] as String?,
          data: (json['d'] as Map?)?.cast<String, Object?>() ?? const {},
        );
      }
    } on FormatException {
      // A line cut short by a crash.
    }
    return null;
  }

  /// Deletes every log file; logging carries on in a fresh one.
  static Future<void> clear() async {
    final dir = _dir;
    if (dir == null) return;
    await _file?.truncate(0);
    for (final name in _files.where((name) => name != 'app.log')) {
      final file = File('${dir.path}/$name');
      if (file.existsSync()) await file.delete();
    }
  }
}

final _secret = RegExp(
  r'''((?<![a-z])(?:password|passwd|otp|csrf|cookie|token|secret|authorization|jsessionid|serverid)["']?\s*[:=]\s*["']?)[^\s,;&"'}]+''',
  caseSensitive: false,
);

/// [text] with the value after anything like `password=` or `otp:` blanked.
String scrub(String text) =>
    text.replaceAllMapped(_secret, (match) => '${match[1]}…');

/// A [Log] with its source fixed, for files that log a lot.
class SourceLogger {
  const SourceLogger(this.source);

  final String source;

  void debug(String message) => Log.debug(source, message);
  void info(String message) => Log.info(source, message);
  void warn(String message) => Log.warn(source, message);
  void error(String message, [StackTrace? stack]) =>
      Log.error(source, message, stack);
}

final _flowKey = Object();
final _random = Random();

/// The id of the flow the current code runs in, if any.
String? get currentFlow => Zone.current[_flowKey] as String?;

/// Runs [body] as one user action or background run named [name]: makes a
/// flow id that every log line inside carries (and that server requests send
/// as `x-request-id`), and logs how long it took. Inside another flow it is
/// a timed step of that flow and keeps its id.
Future<T> flow<T>(
  String name,
  Future<T> Function() body, {
  String source = 'flow',
}) async {
  if (currentFlow != null) return _timed(name, source, body);
  final id = List.generate(
    8,
    (_) => _random.nextInt(16).toRadixString(16),
  ).join();
  return runZoned(() => _timed(name, source, body), zoneValues: {_flowKey: id});
}

Future<T> _timed<T>(
  String name,
  String source,
  Future<T> Function() body,
) async {
  final watch = Stopwatch()..start();
  Log.write(.info, source, '$name start', {'step': name, 'ev': 'start'});
  try {
    final result = await body();
    final ms = watch.elapsedMilliseconds;
    Log.write(.info, source, '$name done ${ms}ms', {
      'step': name,
      'ev': 'done',
      'ms': ms,
    });
    return result;
  } catch (error) {
    final ms = watch.elapsedMilliseconds;
    Log.write(.warn, source, '$name failed ${ms}ms: $error', {
      'step': name,
      'ev': 'failed',
      'ms': ms,
    });
    rethrow;
  }
}
