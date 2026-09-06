import 'package:flutter_inappwebview/flutter_inappwebview.dart';

/// Cookie state belongs to exactly one account. No credentials are kept.
class VtopWebviewStore {
  VtopWebviewStore({Future<void> Function()? clearCookies})
    : _clearCookies = clearCookies ?? clearVtopCookies;

  final Future<void> Function() _clearCookies;
  Future<void> _operations = Future.value();

  /// Cookie updates and account cleanup must never interleave.
  Future<T> serialize<T>(Future<T> Function() action) {
    final result = _operations.then((_) => action());
    _operations = result.then<void>(
      (_) {},
      onError: (Object _, StackTrace _) {},
    );
    return result;
  }

  String? _owner;
  int generation = 0;

  void acquire(String username) {
    final owner = username.toUpperCase();
    if (_owner != null && _owner != owner) {
      throw StateError(
        'Previous VTOP view must be cleared before switching users.',
      );
    }
    _owner = owner;
  }

  Future<void> clearIfDifferent(String? username) async {
    if (_owner != null && _owner != username?.toUpperCase()) await clear();
  }

  Future<void> clear({String? username}) async {
    if (username != null && _owner != username.toUpperCase()) return;
    if (_owner == null) return;
    _owner = null;
    generation++;
    await serialize(_clearCookies);
  }
}

Future<void> clearVtopCookies() async {
  final manager = CookieManager.instance();
  final url = WebUri('https://vtop.vitap.ac.in/vtop/');
  for (final domain in [
    '.vitap.ac.in',
    'vtop.vitap.ac.in',
    '.vtop.vitap.ac.in',
  ]) {
    for (final path in ['/vtop', '/vtop/', '/']) {
      await manager.deleteCookies(url: url, domain: domain, path: path);
    }
  }
}

final vtopWebviewStore = VtopWebviewStore();
