/// How the app names and describes one VTOP page or action, once, for
/// every backend ([LocalVtopBackend] today) to read the same way.
library;

import 'dart:convert';

import 'package:vitapmate/src/api/vtop/vtop_client.dart';

/// A page's key: `name` alone, or `name?k=v&…` sorted by `k`, each side
/// percent-encoded byte by byte except `A–Z a–z 0–9 - . _ ~`. Checked
/// against `rust/vtop-shared/tests/fixtures/page_keys.json`.
String pageKey(String name, [Map<String, String> args = const {}]) {
  if (args.isEmpty) return name;
  final keys = args.keys.toList()..sort();
  final pairs = [for (final k in keys) '${_encode(k)}=${_encode(args[k]!)}'];
  return '$name?${pairs.join('&')}';
}

String _encode(String text) => [
  for (final byte in utf8.encode(text))
    switch (byte) {
      >= 0x30 && <= 0x39 ||
      >= 0x41 && <= 0x5A ||
      >= 0x61 && <= 0x7A ||
      0x2D ||
      0x2E ||
      0x5F ||
      0x7E => String.fromCharCode(byte),
      _ => '%${byte.toRadixString(16).toUpperCase().padLeft(2, '0')}',
    },
].join();

/// A page to read: named by [key], read on the phone by [local].
class VtopQuery<T> {
  const VtopQuery({
    required this.name,
    this.semester,
    this.args = const {},
    required this.fromJson,
    required this.local,
  });

  final String name;

  /// The semester the page belongs to; null for pages that are not per
  /// semester.
  final String? semester;
  final Map<String, String> args;
  final T Function(Map<String, dynamic> json) fromJson;
  final Future<T> Function(VtopClient client) local;

  String get key => pageKey(name, args);
}

/// Something that changes VTOP (an outing apply or cancel): never cached,
/// and sent again only when VTOP refused the session before running it.
class VtopAction<T> {
  const VtopAction({
    required this.name,
    this.semester,
    this.args = const {},
    required this.fromJson,
    required this.local,
  });

  final String name;
  final String? semester;
  final Map<String, String> args;
  final T Function(Map<String, dynamic> json) fromJson;
  final Future<T> Function(VtopClient client) local;

  String get key => pageKey(name, args);
}
