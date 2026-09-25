/// Converts vtop-server JSON (Rust serde: snake_case keys, `update_time` as
/// a number, `ClassKind` as `"Theory"`) into the shape the generated
/// `fromJson` constructors expect (camelCase keys, `updateTime` as a string,
/// `"theory"`).
Object? rustJsonToDart(Object? value, [String? key]) {
  if (value is Map) {
    return {
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
  return buffer.toString();
}
