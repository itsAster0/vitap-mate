import 'dart:io';

import 'package:cronet_http/cronet_http.dart';
import 'package:cupertino_http/cupertino_http.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:http/io_client.dart';
import 'package:vitapmate/core/utils/app_urls.dart';

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
      _buildCronetEngine(),
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

CronetEngine _buildCronetEngine() => CronetEngine.build(
  cacheMode: CacheMode.disabled,
  enableHttp2: true,
  enableQuic: true,
  // Try HTTP/3 to the default server from the first connection.
  // Otherwise Cronet only learns it from Alt-Svc after opening an
  // HTTP/2 connection, which it then keeps reusing.
  quicHints: [
    if (Uri.tryParse(defaultServerUrl)?.host case final host?
        when host.isNotEmpty)
      (host, 443, 443),
  ],
);

/// The Cronet engine behind [sharedAppHttpClient] on Android, null
/// elsewhere. Kept for its net log (see `cronet_net_log.dart`).
final CronetEngine? sharedCronetEngine = Platform.isAndroid
    ? _buildCronetEngine()
    : null;

/// One client per isolate, built on first use and never closed, so
/// connections are reused. Code without a Ref (background isolates, ad hoc
/// services) uses this too; a Cronet engine per call would be costly.
final http.Client sharedAppHttpClient = switch (sharedCronetEngine) {
  final engine? => CronetClient.fromCronetEngine(engine),
  null => createAppHttpClient(),
};

/// [sharedAppHttpClient], overridable in tests.
final appHttpClientProvider = Provider<http.Client>(
  (ref) => sharedAppHttpClient,
);
