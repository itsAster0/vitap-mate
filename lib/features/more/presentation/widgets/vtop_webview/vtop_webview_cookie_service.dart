import 'dart:io';

import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:webview_flutter/webview_flutter.dart';

final _androidCookieManager = WebViewCookieManager();

class VtopWebviewSession {
  const VtopWebviewSession({
    required this.cookies,
    required this.headers,
    required this.userAgent,
  });

  final List<VtopWebviewCookie> cookies;
  final Map<String, String> headers;
  final String userAgent;

  String get cookieHeader => cookies.map((cookie) => cookie.header).join(' ');
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

  String get header => '$name=$value;';
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
}) async {
  final cookieManager = CookieManager.instance();
  final cookies = snapshot.cookies
      .where((cookie) => cookie.name.isNotEmpty && cookie.value.isNotEmpty)
      .map(
        (cookie) => VtopWebviewCookie(
          name: cookie.name,
          value: cookie.value,
          domain: cookie.domain.isNotEmpty ? cookie.domain : '.vitap.ac.in',
          path: cookie.path.isNotEmpty ? cookie.path : '/',
          isSecure: cookie.secure,
          expiresDate: cookie.expiresAtEpochMs?.toInt(),
        ),
      )
      .toList(growable: false);
  if (cookies.isEmpty) return null;

  await cookieManager.deleteAllCookies();
  await _androidCookieManager.clearCookies();

  for (final cookie in cookies) {
    await cookieManager.setCookie(
      url: baseUrl,
      name: cookie.name,
      value: cookie.value,
      expiresDate: cookie.expiresDate,
      isSecure: cookie.isSecure,
      domain: cookie.domain,
      path: cookie.path,
    );

    if (Platform.isAndroid) {
      await _androidCookieManager.setCookie(
        WebViewCookie(
          name: cookie.name,
          value: cookie.value,
          domain: cookie.domain,
          path: cookie.path,
        ),
      );
    }
  }

  final headers = <String, String>{};
  for (final header in snapshot.headers) {
    final name = header.name.trim();
    final value = header.value.trim();
    if (name.isEmpty || value.isEmpty) continue;
    headers[name] = value;
  }

  headers['Cookie'] = cookies
      .map((cookie) => '${cookie.name}=${cookie.value}')
      .join('; ');

  final userAgent =
      headers['User-Agent'] ??
      headers['user-agent'] ??
      'Mozilla/5.0 (Linux; U; Linux x86_64; en-US) Gecko/20100101 Firefox/130.5';

  return VtopWebviewSession(
    cookies: cookies,
    headers: headers,
    userAgent: userAgent,
  );
}
