import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:vitapmate/core/utils/vtop_webview_pages.dart';

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
  String? _username;
  String? get username => _username;
  int generation = 0;

  var _keepAlive = InAppWebViewKeepAlive();

  /// Cookies the kept-alive page was opened with, or null before any load.
  String? keptCookies;

  /// Pages visited in the kept view, oldest first, for the back button.
  final history = <VtopPage>[];

  /// The sidebar's pages, read once per kept view.
  List<VtopPage>? pages;

  /// Reopening VTOP reattaches this native view instead of starting over.
  InAppWebViewKeepAlive get keepAlive => _keepAlive;

  /// Forgets what the kept view showed, so the next open loads Home in it.
  void resetKeptView() {
    keptCookies = null;
    history.clear();
    pages = null;
  }

  /// A view whose page process died cannot be reused. The old one is never
  /// disposed: flutter_inappwebview 6.1.5 leaves a null in its native map
  /// after disposeKeepAlive and then crashes when the activity is destroyed
  /// (upstream #2025). Android releases the dead view with the activity.
  void replaceKeptView() {
    resetKeptView();
    _keepAlive = InAppWebViewKeepAlive();
  }

  void acquire(String username) {
    final owner = username.toUpperCase();
    if (_owner != null && _owner != owner) {
      throw StateError(
        'Previous VTOP view must be cleared before switching users.',
      );
    }
    _owner = owner;
    _username = username;
  }

  Future<void> clearIfDifferent(String? username) async {
    if (_owner != null && _owner != username?.toUpperCase()) await clear();
  }

  Future<void> clear({String? username}) async {
    if (username != null && _owner != username.toUpperCase()) return;
    if (_owner == null) return;
    _owner = null;
    _username = null;
    generation++;
    // The kept view stays alive (see replaceKeptView) but is forgotten, so
    // the next account starts from a fresh Home load with its own cookies.
    resetKeptView();
    await serialize(_clearCookies);
  }
}

Future<void> clearVtopCookies() async {
  // The in-app web view only ever visits VTOP, so its whole jar is VTOP's.
  await CookieManager.instance().deleteAllCookies();
}

final vtopWebviewStore = VtopWebviewStore();
