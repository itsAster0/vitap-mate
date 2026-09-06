import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:vitapmate/src/api/vtop/types.dart';

class VtopWebviewSession {
  const VtopWebviewSession({required this.cookies});

  final List<VtopWebviewCookie> cookies;

  String get cookieHeader => cookies.map((cookie) => cookie.header).join('; ');
}

class VtopWebviewCookie {
  const VtopWebviewCookie({
    required this.name,
    required this.value,
    required this.domain,
    required this.path,
    required this.isSecure,
    this.expiresDate,
  });

  final String name;
  final String value;
  final String domain;
  final String path;
  final bool isSecure;
  final int? expiresDate;

  String get header => '$name=$value';
}

Future<String> loadLatestVtopCookieHeader(
  WebUri url, {
  VtopWebviewSession? fallbackSession,
}) async {
  final cookieManager = CookieManager.instance();
  final cookies = await cookieManager.getCookies(url: url);
  if (cookies.isNotEmpty) {
    return cookies
        .where((cookie) => cookie.name.isNotEmpty && cookie.value.isNotEmpty)
        .map((cookie) => '${cookie.name}=${cookie.value}')
        .join('; ');
  }

  return fallbackSession?.cookieHeader ?? '';
}

Future<VtopWebviewSession?> loadVtopWebviewSession({
  required PersistedVtopSession snapshot,
  required WebUri baseUrl,
  CookieManager? manager,
  bool Function()? isCurrent,
}) async {
  final cookieManager = manager ?? CookieManager.instance();
  if (isCurrent?.call() == false) return null;
  final parsedCookies = <({String name, String value})>[];
  for (final cookiePart in (snapshot.cookies ?? '').split(';')) {
    final parsedCookie = parseCookiePair(cookiePart);
    if (parsedCookie == null) continue;
    parsedCookies.add(parsedCookie);
  }
  final path = '/vtop';
  final cookies = parsedCookies
      .map(
        (cookie) => VtopWebviewCookie(
          name: cookie.name,
          value: cookie.value,
          domain: '.vitap.ac.in',
          path: path,
          isSecure: true,
        ),
      )
      .toList(growable: false);
  if (cookies.isEmpty) return null;

  final cookieUrl = WebUri('${baseUrl.origin}/vtop/');
  final existing = await cookieManager.getCookies(url: cookieUrl);
  final desiredNames = cookies.map((cookie) => cookie.name).toSet();
  for (final old in existing) {
    if (isCurrent?.call() == false) return null;
    final matching = cookies.where((cookie) => cookie.name == old.name);
    if (desiredNames.contains(old.name) &&
        matching.any(
          (cookie) =>
              cookie.value == old.value &&
              (old.path == null || old.path == cookie.path),
        )) {
      continue;
    }
    // Android may return only names and values. Remove stale variants at the
    // known VTOP scopes instead of deleting cookies for unrelated websites.
    final domains = old.domain == null
        ? ['.vitap.ac.in', 'vtop.vitap.ac.in', '.vtop.vitap.ac.in']
        : [old.domain!];
    final paths = old.path == null ? ['/vtop', '/vtop/', '/'] : [old.path!];
    for (final domain in domains) {
      for (final oldPath in paths) {
        if (isCurrent?.call() == false) return null;
        await cookieManager.deleteCookie(
          url: cookieUrl,
          name: old.name,
          domain: domain,
          path: oldPath,
        );
      }
    }
  }
  for (final cookie in cookies) {
    if (isCurrent?.call() == false) return null;
    if (existing.any(
      (old) =>
          old.name == cookie.name &&
          old.value == cookie.value &&
          (old.path == null || old.path == cookie.path),
    )) {
      continue;
    }
    final didSet = await cookieManager.setCookie(
      url: cookieUrl,
      name: cookie.name,
      value: cookie.value,
      isSecure: cookie.isSecure,
      domain: cookie.domain,
      path: cookie.path,
    );
    if (!didSet) throw StateError('Could not synchronize VTOP cookies.');
  }

  return VtopWebviewSession(cookies: cookies);
}

({String name, String value})? parseCookiePair(String cookieHeader) {
  final firstCookiePart = cookieHeader.split(';').first.trim();
  final separatorIndex = firstCookiePart.indexOf('=');
  if (separatorIndex <= 0 || separatorIndex >= firstCookiePart.length - 1) {
    return null;
  }

  final name = firstCookiePart.substring(0, separatorIndex).trim();
  final value = firstCookiePart.substring(separatorIndex + 1).trim();
  if (name.isEmpty || value.isEmpty) {
    return null;
  }

  return (name: name, value: value);
}
