class LogEntry {
  const LogEntry({
    required this.level,
    required this.source,
    required this.message,
    required this.timestamp,
    this.error,
    this.stackTrace,
    this.tags = const <String>[],
  });

  final LogLevel level;
  final String source;
  final String message;
  final DateTime timestamp;
  final String? error;
  final String? stackTrace;
  final List<String> tags;

  Map<String, dynamic> toJson() {
    return {
      'level': level.name,
      'source': source,
      'message': message,
      'timestamp': timestamp.toIso8601String(),
      'error': error,
      'stackTrace': stackTrace,
      'tags': tags,
    };
  }

  factory LogEntry.fromJson(Map<String, dynamic> json) {
    return LogEntry(
      level: LogLevel.values.firstWhere(
        (level) => level.name == json['level'],
        orElse: () => LogLevel.info,
      ),
      source: json['source'] as String? ?? 'app',
      message: json['message'] as String? ?? '',
      timestamp:
          DateTime.tryParse(json['timestamp'] as String? ?? '') ??
          DateTime.now(),
      error: json['error'] as String?,
      stackTrace: json['stackTrace'] as String?,
      tags: (json['tags'] as List<dynamic>? ?? const <dynamic>[])
          .map((tag) => tag.toString())
          .toList(growable: false),
    );
  }
}

enum LogLevel { trace, debug, info, warn, error }
