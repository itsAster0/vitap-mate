import 'dart:io';

import 'package:flutter_inappwebview/flutter_inappwebview.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/src/api/vtop_get_client.dart';
import 'package:webview_flutter/webview_flutter.dart';

final _androidCookieManager = WebViewCookieManager();

class VtopWebviewCookie {
  const VtopWebviewCookie({required this.name, required this.value});

  final String name;
  final String value;

  String get header => '$name=$value;';
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

Future<VtopWebviewCookie?> loadVtopWebviewCookie({
  required WidgetRef ref,
  required WebUri baseUrl,
}) async {
  final cookieManager = CookieManager.instance();
  final expiresDate =
      DateTime.now().add(const Duration(minutes: 30)).millisecondsSinceEpoch;

  final client = await ref.read(vClientProvider.future);
  final rawCookie = await fetchCookies(client: client);
  final cookieString = String.fromCharCodes(rawCookie);
  final parsedCookie = parseCookiePair(cookieString);
  if (parsedCookie == null) return null;

  await cookieManager.deleteAllCookies();
  await _androidCookieManager.clearCookies();

  if (Platform.isAndroid) {
    await _androidCookieManager.setCookie(
      WebViewCookie(
        name: parsedCookie.name,
        value: parsedCookie.value,
        domain: '.vitap.ac.in',
        path: '/',
      ),
    );
  } else {
    await cookieManager.setCookie(
      url: baseUrl,
      name: parsedCookie.name,
      value: parsedCookie.value,
      expiresDate: expiresDate,
      isSecure: true,
      domain: '.vitap.ac.in',
      path: '/',
    );
  }

  return VtopWebviewCookie(name: parsedCookie.name, value: parsedCookie.value);
}
