import 'dart:io';

import 'package:cronet_http/cronet_http.dart';
import 'package:cupertino_http/cupertino_http.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:http/io_client.dart';
import 'package:path_provider/path_provider.dart';
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

/// Cronet's QUIC options for network changes (Wi-Fi to mobile data, or a
/// VPN starting). By default a change kills the QUIC connection, the
/// requests on it fail over to TCP and QUIC is marked broken for this host
/// for 5+ minutes, so everything runs on HTTP/2 until then. With these, the
/// connection moves to the new default network instead, idle ones too.
/// Nothing moves to mobile data on its own: no early migration while Wi-Fi
/// is merely degraded, and no handshake retry on the other network.
const _quicNetworkChangeOptions = '''
{"QUIC": {
  "migrate_sessions_on_network_change_v2": true,
  "migrate_idle_sessions": true,
  "idle_session_migration_period_seconds": 30,
  "migrate_sessions_early_v2": false,
  "retry_on_alternate_network_before_handshake": false,
  "goaway_sessions_on_ip_change": false,
  "close_sessions_on_ip_change": false
}}''';

/// Where the main isolate's engine keeps what it learns about servers
/// (which speak QUIC, how fast they answer, 0-RTT keys), so a cold start
/// goes straight to HTTP/3 instead of racing TCP. Set by
/// [useAppHttpStorage] before the first request. Other isolates leave it
/// null and keep nothing, as Cronet needs one directory per engine. Cronet
/// keeps no cookies there (integration_test/cronet_cookies_test.dart).
String? _storagePath;

/// Gives this isolate's engine [_storagePath]. Called once by the app's
/// entry point, before anything uses [sharedAppHttpClient].
Future<void> useAppHttpStorage() async {
  if (!Platform.isAndroid) return;
  try {
    final dir = Directory(
      '${(await getApplicationSupportDirectory()).path}/cronet',
    );
    await dir.create(recursive: true);
    _storagePath = dir.path;
  } catch (_) {
    // Without it the engine only remembers servers until the app exits.
  }
}

CronetEngine _buildCronetEngine({String? storagePath}) => CronetEngine.build(
  cacheMode: storagePath == null ? CacheMode.disabled : CacheMode.diskNoHttp,
  // Required with a disk mode; no HTTP responses are cached in it.
  cacheMaxSize: storagePath == null ? null : 1 << 20,
  storagePath: storagePath,
  // Cloudflare answers Brotli when offered; Cronet decodes it, so callers
  // still read plain bodies.
  enableBrotli: true,
  // With storage, DNS answers are kept across launches too, so a cold start
  // need not wait on a lookup, and the last known address is used when a
  // lookup fails on a flaky network.
  persistHostCache: storagePath != null ? true : null,
  enableStaleDns: storagePath != null ? true : null,
  useStaleOnNameNotResolved: storagePath != null ? true : null,
  enableHttp2: true,
  enableQuic: true,
  experimentalOptions: _quicNetworkChangeOptions,
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
    ? _buildCronetEngine(storagePath: _storagePath)
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
