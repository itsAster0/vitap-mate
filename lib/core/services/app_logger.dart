import 'dart:async';
import 'dart:convert';
import 'dart:developer' as developer;
import 'dart:io';

import 'package:vitapmate/core/models/log_entry.dart';

class AppLogger {
  AppLogger(this._logFile);

  final File _logFile;
  final List<LogEntry> _entries = <LogEntry>[];
  final StreamController<List<LogEntry>> _updates =
      StreamController<List<LogEntry>>.broadcast();
  bool _initialized = false;

  List<LogEntry> get entries => List<LogEntry>.unmodifiable(_entries);
  Stream<List<LogEntry>> get stream => _updates.stream;

  Future<void> initialize() async {
    if (_initialized) return;
    _initialized = true;
    if (await _logFile.exists()) {
      try {
        final decoded = jsonDecode(await _logFile.readAsString());
        if (decoded is List) {
          _entries
            ..clear()
            ..addAll(
              decoded.whereType<Map>().map(
                (entry) => LogEntry.fromJson(
                  entry.map((key, value) => MapEntry(key.toString(), value)),
                ),
              ),
            );
        }
      } catch (_) {
        _entries.clear();
      }
    }
    _updates.add(entries);
  }

  void trace(
    String source,
    String message, {
    Object? error,
    StackTrace? stackTrace,
    List<String> tags = const <String>[],
  }) => log(
    LogLevel.trace,
    source,
    message,
    error: error,
    stackTrace: stackTrace,
    tags: tags,
  );

  void debug(
    String source,
    String message, {
    Object? error,
    StackTrace? stackTrace,
    List<String> tags = const <String>[],
  }) => log(
    LogLevel.debug,
    source,
    message,
    error: error,
    stackTrace: stackTrace,
    tags: tags,
  );

  void info(
    String source,
    String message, {
    Object? error,
    StackTrace? stackTrace,
    List<String> tags = const <String>[],
  }) => log(
    LogLevel.info,
    source,
    message,
    error: error,
    stackTrace: stackTrace,
    tags: tags,
  );

  void warn(
    String source,
    String message, {
    Object? error,
    StackTrace? stackTrace,
    List<String> tags = const <String>[],
  }) => log(
    LogLevel.warn,
    source,
    message,
    error: error,
    stackTrace: stackTrace,
    tags: tags,
  );

  void error(
    String source,
    String message, {
    Object? error,
    StackTrace? stackTrace,
    List<String> tags = const <String>[],
  }) => log(
    LogLevel.error,
    source,
    message,
    error: error,
    stackTrace: stackTrace,
    tags: tags,
  );

  void log(
    LogLevel level,
    String source,
    String message, {
    Object? error,
    StackTrace? stackTrace,
    List<String> tags = const <String>[],
  }) {
    final entry = LogEntry(
      level: level,
      source: source,
      message: message,
      timestamp: DateTime.now(),
      error: error?.toString(),
      stackTrace: stackTrace?.toString(),
      tags: tags,
    );
    developer.log(
      message,
      name: source,
      level: _developerLevel(level),
      error: error,
      stackTrace: stackTrace,
    );
    _entries.insert(0, entry);
    if (_entries.length > 500) {
      _entries.removeRange(500, _entries.length);
    }
    unawaited(_persist());
    _updates.add(entries);
  }

  Future<void> clear() async {
    _entries.clear();
    await _persist();
    _updates.add(entries);
  }

  Future<void> _persist() async {
    await _logFile.parent.create(recursive: true);
    await _logFile.writeAsString(
      jsonEncode(_entries.map((entry) => entry.toJson()).toList()),
      flush: true,
    );
  }

  int _developerLevel(LogLevel level) {
    return switch (level) {
      LogLevel.trace => 300,
      LogLevel.debug => 500,
      LogLevel.info => 800,
      LogLevel.warn => 900,
      LogLevel.error => 1000,
    };
  }
}
