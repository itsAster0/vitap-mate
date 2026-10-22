// Run on an Android device or emulator, online:
//   flutter test integration_test/cronet_cookies_test.dart -d <device>
//
// Proxied VTOP requests share the app's Cronet engine on the promise that
// Cronet keeps no cookies of its own: vtop-core sends each session's cookies
// itself. This checks that promise, with and without disk storage.

import 'dart:convert';
import 'dart:io';

import 'package:cronet_http/cronet_http.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';

/// The cookies httpbin says the request carried.
Future<Map<String, dynamic>> _sentCookies(
  http.Client client, {
  String? cookieHeader,
}) async {
  final response = await client.get(
    Uri.parse('https://httpbin.org/cookies'),
    headers: {'Cookie': ?cookieHeader},
  );
  return (jsonDecode(response.body) as Map<String, dynamic>)['cookies']
      as Map<String, dynamic>;
}

Future<void> _expectNoCookieJar(CronetEngine engine) async {
  final client = CronetClient.fromCronetEngine(engine, closeEngine: true);
  try {
    final set = http.Request(
      'GET',
      Uri.parse('https://httpbin.org/cookies/set?probe=1'),
    )..followRedirects = false;
    final setResponse = await client.send(set);
    await setResponse.stream.drain<void>();
    expect(setResponse.headers['set-cookie'], contains('probe=1'));

    expect(await _sentCookies(client), isEmpty);
    expect(await _sentCookies(client, cookieHeader: 'mine=1'), {'mine': '1'});
  } finally {
    client.close();
  }
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('without storage, Cronet keeps no cookies', (_) async {
    await _expectNoCookieJar(CronetEngine.build(cacheMode: CacheMode.disabled));
  });

  testWidgets('with disk storage, Cronet keeps no cookies', (_) async {
    final dir = Directory(
      '${(await getTemporaryDirectory()).path}/cronet-cookie-test',
    );
    if (await dir.exists()) await dir.delete(recursive: true);
    await dir.create(recursive: true);
    await _expectNoCookieJar(
      CronetEngine.build(
        cacheMode: CacheMode.diskNoHttp,
        cacheMaxSize: 1 << 20,
        storagePath: dir.path,
      ),
    );
  });
}
