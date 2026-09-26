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
  final path = '/vtop';
  final cookies = [
    for (final cookiePart in (snapshot.cookies ?? '').split(';'))
      if (parseCookiePair(cookiePart) case final cookie?)
        VtopWebviewCookie(
          name: cookie.name,
          value: cookie.value,
          domain: '.vitap.ac.in',
          path: path,
          isSecure: true,
        ),
  ];
  if (cookies.isEmpty) return null;

  // Start from an empty jar. Targeted deletes miss VTOP's own host-only
  // JSESSIONID, which then shadows the new session and VTOP keeps sending the
  // view to its login page. Re-setting an unchanged value keeps the session,
  // so a reopened page is unaffected.
  await cookieManager.deleteAllCookies();

  for (final cookie in cookies) {
    if (isCurrent?.call() == false) return null;
    final didSet = await cookieManager.setCookie(
      url: WebUri('${baseUrl.origin}/vtop/'),
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
