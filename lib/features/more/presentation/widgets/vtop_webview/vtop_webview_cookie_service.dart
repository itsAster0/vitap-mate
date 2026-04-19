import 'dart:io';

import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:vitapmate/src/api/vtop/types.dart';
import 'package:webview_flutter/webview_flutter.dart';

final _androidCookieManager = WebViewCookieManager();

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
}) async {
  final cookieManager = CookieManager.instance();
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

  await cookieManager.deleteAllCookies();
  await _androidCookieManager.clearCookies();

  for (final cookie in cookies) {
    if (Platform.isAndroid) {
      await _androidCookieManager.setCookie(
        WebViewCookie(
          name: cookie.name,
          value: cookie.value,
          domain: '.vitap.ac.in',
          path: path,
        ),
      );
    } else {
      await cookieManager.setCookie(
        url: baseUrl,
        name: cookie.name,
        value: cookie.value,
        expiresDate: cookie.expiresDate,
        isSecure: true,
        domain: '.vitap.ac.in',
        path: path,
      );
    }
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
