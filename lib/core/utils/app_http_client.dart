import 'dart:io';

import 'package:cronet_http/cronet_http.dart';
import 'package:cupertino_http/cupertino_http.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:http/io_client.dart';

/// The HTTP client for the VITAP Mate server: Cronet on Android and
/// NSURLSession on iOS, both of which speak HTTP/3 (QUIC). On a
/// weak network QUIC loses less to dropped packets and survives a switch
/// between Wi-Fi and mobile data. Elsewhere, dart:io's client.
/// Which client [createAppHttpClient] makes here, for Diagnostics.
String appHttpClientName() => Platform.isAndroid
    ? 'Cronet (HTTP/3)'
    : Platform.isIOS
    ? 'Cupertino'
    : 'Standard';

http.Client createAppHttpClient() {
  if (Platform.isAndroid) {
    return CronetClient.fromCronetEngine(
      CronetEngine.build(
        cacheMode: CacheMode.disabled,
        enableHttp2: true,
        enableQuic: true,
      ),
      closeEngine: true,
    );
  }
  if (Platform.isIOS) {
    return CupertinoClient.fromSessionConfiguration(
      URLSessionConfiguration.ephemeralSessionConfiguration(),
    );
  }
  return IOClient();
}

/// One client for the app's lifetime, so connections are reused.
final appHttpClientProvider = Provider<http.Client>((ref) {
  final client = createAppHttpClient();
  ref.onDispose(client.close);
  return client;
});
