/// The app's one log: JSON lines in `logs/app.log` on the phone, rolled over
/// to `app.log.1` past [Log.maxBytes], next to Rust's `logs/rust.log` (same
/// format, see `rust/src/api/native_logs.rs`). Every line carries the
/// [flow] it belongs to and the student's [Log.regno]. Nothing leaves the
/// phone unless the student copies or saves it.
library;

import 'dart:async';
import 'dart:convert';
import 'dart:developer' as developer;
import 'dart:io';
import 'dart:math';

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
});

abstract final class Log {
  static const maxBytes = 1024 * 1024;
  static const _files = ['app.log.1', 'app.log', 'rust.log.1', 'rust.log'];

  static Directory? _dir;
  static RandomAccessFile? _file;

  /// The signed-in student's registration number, once known.
  static String? regno;

  /// Opens the log in [dir]. Lines written before this only reach the
  /// debugger. Each isolate opens it for itself.
  static Future<void> open(Directory dir) async {
    await dir.create(recursive: true);
    _dir = dir;
    _file = await File('${dir.path}/app.log').open(mode: FileMode.append);
  }

  static void debug(String source, String message) =>
      write(.debug, source, message);
  static void info(String source, String message) =>
      write(.info, source, message);
  static void warn(String source, String message) =>
      write(.warn, source, message);
  static void error(String source, String message) =>
      write(.error, source, message);

  static void write(LogLevel level, String source, String message) {
    developer.log(message, name: source, level: level.value);
    final line = jsonEncode({
      't': DateTime.now().millisecondsSinceEpoch,
      'l': level.name,
      'src': source,
      'msg': message,
      'flow': ?currentFlow,
      'regno': ?regno,
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

  /// Every line on the phone, Dart's and Rust's, oldest first.
  static Future<List<LogLine>> read() async {
    final dir = _dir;
    if (dir == null) return const [];
    final lines = <LogLine>[
      for (final name in _files)
        if (File('${dir.path}/$name') case final file when file.existsSync())
          for (final raw in await file.readAsLines()) ?_parse(raw),
    ]..sort((a, b) => a.time.compareTo(b.time));
    return lines;
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
          message: message,
          flow: json['flow'] as String?,
          regno: json['regno'] as String?,
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

/// A [Log] with its source fixed, for files that log a lot.
class SourceLogger {
  const SourceLogger(this.source);

  final String source;

  void debug(String message) => Log.debug(source, message);
  void info(String message) => Log.info(source, message);
  void warn(String message) => Log.warn(source, message);
  void error(String message) => Log.error(source, message);
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
  Log.info(source, '$name start');
  try {
    final result = await body();
    Log.info(source, '$name done ${watch.elapsedMilliseconds}ms');
    return result;
  } catch (error) {
    Log.warn(source, '$name failed ${watch.elapsedMilliseconds}ms: $error');
    rethrow;
  }
}
