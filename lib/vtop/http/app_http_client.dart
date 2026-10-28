import 'dart:convert';
import 'dart:io';

import 'package:cronet_http/cronet_http.dart';
import 'package:cupertino_http/cupertino_http.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:http/http.dart' as http;
import 'package:http/io_client.dart';
import 'package:meta/meta.dart';
import 'package:path_provider/path_provider.dart';
import 'package:vitapmate/app/app_urls.dart';
import 'package:vitapmate/vtop/http/cronet_net_log.dart';

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
  "idle_connection_timeout_seconds": 60,
  "migrate_sessions_on_network_change_v2": true,
  "migrate_idle_sessions": true,
  "idle_session_migration_period_seconds": 60,
  "migrate_sessions_early_v2": false,
  "retry_on_alternate_network_before_handshake": false,
  "goaway_sessions_on_ip_change": false,
  "close_sessions_on_ip_change": false
}}''';

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

/// This isolate's engine and client on Android, built on the first request
/// rather than at launch.
final Future<({CronetEngine engine, http.Client client})> _cronet =
    _buildCronet();

Future<({CronetEngine engine, http.Client client})> _buildCronet() async {
  final storage = await _claimStorage();
  final engine = _buildCronetEngine(storagePath: storage);
  // The main isolate's engine only (the one with storage).
  if (storage != null) await CronetNetLog.recordLaunch(engine);
  return (engine: engine, client: CronetClient.fromCronetEngine(engine));
}

/// Where the engine keeps what it learns about servers (which speak QUIC,
/// how fast they answer, DNS answers), so a cold start goes straight to
/// HTTP/3. Cronet keeps no cookies there
/// (integration_test/cronet_cookies_test.dart).
///
/// Cronet needs one directory per engine, so only one isolate per process
/// may use it: the first to send a request claims it by writing the process
/// id into `cronet.owner` beside it. Other isolates (push and background
/// sync, while the app runs) get null and keep what they learn in memory. A
/// claim left by an earlier process is taken over.
Future<String?> _claimStorage() async {
  try {
    final root = Directory(await _storageRoot());
    await root.create(recursive: true);
    final owner = File('${root.path}.owner');
    if (await owner.exists()) {
      if (int.tryParse(await owner.readAsString()) == pid) return null;
      await owner.delete();
    }
    // Exclusive: of two isolates racing here, one wins.
    await owner.create(exclusive: true);
    await owner.writeAsString('$pid');
    final forget = File('${root.path}.forget-quic');
    if (await forget.exists()) {
      await _forgetBrokenQuic(File('${root.path}/prefs/local_prefs.json'));
      await forget.delete();
    }
    return root.path;
  } catch (_) {
    // Without it the engine only remembers servers until the app exits.
    return null;
  }
}

Future<String> _storageRoot() async =>
    '${(await getApplicationSupportDirectory()).path}/cronet';

/// Has the next launch forget the QUIC failures Cronet saved, before its
/// engine starts. When QUIC failed for a reason now gone (a VPN dropping
/// UDP), Cronet keeps to HTTP/2 for minutes to hours, across restarts. Not
/// done in this run: the live engine holds the failures in memory and would
/// save them again.
Future<void> forgetQuicFailuresOnNextLaunch() async {
  await File('${await _storageRoot()}.forget-quic').create(recursive: true);
}

Future<void> _forgetBrokenQuic(File prefs) async {
  if (!await prefs.exists()) return;
  try {
    final json = jsonDecode(await prefs.readAsString());
    if (withoutBrokenAlternativeServices(json)) {
      await prefs.writeAsString(jsonEncode(json));
    }
  } catch (_) {
    // Unreadable prefs: Cronet starts over without them anyway.
  }
}

/// Drops the saved QUIC failures from Cronet's prefs [json] in place,
/// keeping the rest (which servers speak QUIC, their round-trip times, DNS
/// answers). Whether there were any.
@visibleForTesting
bool withoutBrokenAlternativeServices(Object? json) {
  if (json case {'net': {'http_server_properties': final Map props}}) {
    return props.remove('broken_alternative_services') != null;
  }
  return false;
}

/// The engine behind [sharedAppHttpClient] on Android once built, for its
/// net log (see `cronet_net_log.dart`); null elsewhere.
Future<CronetEngine?> get sharedCronetEngine async =>
    Platform.isAndroid ? (await _cronet).engine : null;

/// One client per isolate, set up on its first request and never closed, so
/// connections are reused. Code without a Ref (background isolates, ad hoc
/// services) uses this too; a Cronet engine per call would be costly.
final http.Client sharedAppHttpClient = Platform.isAndroid
    ? _DeferredClient(_cronet.then((cronet) => cronet.client))
    : createAppHttpClient();

/// Sends through the client [_client] completes with.
class _DeferredClient extends http.BaseClient {
  _DeferredClient(this._client);

  final Future<http.Client> _client;

  @override
  Future<http.StreamedResponse> send(http.BaseRequest request) async =>
      (await _client).send(request);

  @override
  void close() => _client.then((client) => client.close());
}

/// [sharedAppHttpClient], overridable in tests.
final appHttpClientProvider = Provider<http.Client>(
  (ref) => sharedAppHttpClient,
);
