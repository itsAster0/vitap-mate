import 'dart:collection';
import 'dart:developer' as developer;

import 'package:flutter/foundation.dart';

enum AppLogLevel {
  debug('DEBUG', 500),
  info('INFO', 800),
  warning('WARN', 900),
  error('ERROR', 1000);

  const AppLogLevel(this.label, this.value);

  final String label;

  /// The level for `dart:developer` logs.
  final int value;
}

class AppLogEntry {
  final DateTime timestamp;
  final AppLogLevel level;
  final String source;
  final String message;

  const AppLogEntry({
    required this.timestamp,
    required this.level,
    required this.source,
    required this.message,
  });

  String get levelLabel => level.label;

  String toLine() =>
      '[${timestamp.toIso8601String()}][$levelLabel][$source] $message';
}

class AppLogger extends ChangeNotifier {
  AppLogger._();

  static final AppLogger instance = AppLogger._();

  static const int _maxEntries = 500;

  final List<AppLogEntry> _entries = <AppLogEntry>[];

  UnmodifiableListView<AppLogEntry> get entries =>
      UnmodifiableListView<AppLogEntry>(_entries);

  void debug(String source, String message) =>
      _add(AppLogLevel.debug, source, message);

  void info(String source, String message) =>
      _add(AppLogLevel.info, source, message);

  void warning(String source, String message) =>
      _add(AppLogLevel.warning, source, message);

  void error(String source, String message) =>
      _add(AppLogLevel.error, source, message);

  Future<T> trackRequest<T>({
    required String source,
    required String action,
    required Future<T> Function() run,
  }) async {
    final stopwatch = Stopwatch()..start();
    info(source, 'request.start $action');
    try {
      final result = await run();
      info(
        source,
        'request.success $action (${stopwatch.elapsedMilliseconds}ms)',
      );
      return result;
    } catch (error) {
      this.error(
        source,
        'request.error $action (${stopwatch.elapsedMilliseconds}ms): $error',
      );
      rethrow;
    }
  }

  void clear() {
    _entries.clear();
    notifyListeners();
  }

  void _add(AppLogLevel level, String source, String message) {
    final entry = AppLogEntry(
      timestamp: DateTime.now(),
      level: level,
      source: source,
      message: message,
    );
    // Logged twice, as the full line and as the bare message.
    for (final text in [entry.toLine(), message]) {
      developer.log(
        text,
        name: source,
        level: level.value,
        time: entry.timestamp,
      );
    }
    _entries.insert(0, entry);
    if (_entries.length > _maxEntries) {
      _entries.removeRange(_maxEntries, _entries.length);
    }
    notifyListeners();
  }
}

/// [AppLogger] with the source fixed, for one-line calls.
class SourceLogger {
  const SourceLogger(this.source);

  final String source;

  void info(String message) => AppLogger.instance.info(source, message);
  void warning(String message) => AppLogger.instance.warning(source, message);
  void error(String message) => AppLogger.instance.error(source, message);
}
