import 'package:vitapmate/src/api/vtop/types.dart';

/// Converts the engine's JSON (Rust serde: snake_case keys, `update_time`
/// as a number, `ClassKind` as `"Theory"`) into the shape the generated
/// `fromJson` constructors expect (camelCase keys, `updateTime` as a
/// string, `"theory"`). Checked against vtop-core's parser snapshots.
Object? rustJsonToDart(Object? value, [String? key]) {
  if (value is Map) {
    return <String, dynamic>{
      for (final entry in value.entries)
        _camelCase('${entry.key}'): rustJsonToDart(entry.value, '${entry.key}'),
    };
  }
  if (value is List) {
    return [for (final item in value) rustJsonToDart(item)];
  }
  if (key == 'update_time' && value is num) return value.toInt().toString();
  if (key == 'kind' && value is String) return value.toLowerCase();
  return value;
}

String _camelCase(String snake) {
  final parts = snake.split('_');
  final buffer = StringBuffer(parts.first);
  for (final part in parts.skip(1)) {
    if (part.isEmpty) continue;
    buffer
      ..write(part[0].toUpperCase())
      ..write(part.substring(1));
  }
  final name = buffer.toString();
  // flutter_rust_bridge names such fields `class_`, `default_`, …
  return _reserved.contains(name) ? '${name}_' : name;
}

/// Dart's reserved words, which cannot be field names.
const _reserved = {
  'assert', 'break', 'case', 'catch', 'class', 'const', 'continue', //
  'default', 'do', 'else', 'enum', 'extends', 'false', 'final', 'finally',
  'for', 'if', 'in', 'is', 'new', 'null', 'rethrow', 'return', 'super',
  'switch', 'this', 'throw', 'true', 'try', 'var', 'void', 'while', 'with',
};

/// [SessionState] in the engine's JSON shape.
Map<String, Object?> sessionToServerJson(SessionState session) => {
  'cookies': session.cookies,
  if (session.csrfToken != null) 'csrf_token': session.csrfToken,
  if (session.registrationNumber != null)
    'registration_number': session.registrationNumber,
  if (session.otpIssuedAt != null)
    'otp_issued_at': session.otpIssuedAt!.toInt(),
  if (session.loggedInAt != null) 'logged_in_at': session.loggedInAt!.toInt(),
};

/// A session the engine sent back.
SessionState sessionFromServerJson(Map<String, dynamic> json) {
  BigInt? number(Object? value) =>
      value is num ? BigInt.from(value.toInt()) : null;
  return SessionState(
    cookies: '${json['cookies'] ?? ''}',
    csrfToken: json['csrf_token'] as String?,
    registrationNumber: json['registration_number'] as String?,
    otpIssuedAt: number(json['otp_issued_at']),
    loggedInAt: number(json['logged_in_at']),
  );
}
