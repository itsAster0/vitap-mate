/// The experimental UDP mode for "Fetch on the server"
/// ([VtopServerMode.server]): pages come from the UDP gateway
/// (`rust/vtop-gateway`) over the app's own transport (`rust/vtop-wire`),
/// built for slow, lossy networks where HTTPS spends most of its time on
/// handshakes and retransmission timers. Sign-in, OTP and changes to VTOP
/// still go over HTTPS, and so does every page when the gateway cannot be
/// reached. Where the gateway is and its key come from the server
/// ([currentUdpGateway]), so a move or a new key needs no release.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/app/settings.dart';
import 'package:vitapmate/observability/log.dart';
import 'package:vitapmate/src/api/wire.dart';
import 'package:vitapmate/vtop/http/app_http_client.dart';
import 'package:vitapmate/vtop/route/server_url.dart';

const vtopUdpSettingKey = 'settings_vtop_udp';

const _gatewaySettingKey = 'settings_udp_gateway';

/// Where the gateway is and its public key, as the server's `GET /v1/udp`
/// answers. Kept until [expiresAt] (unix seconds), the same Monday 00:00 IST
/// for every phone; then asked again.
typedef UdpGateway = ({
  String host,
  List<int> ports,
  String publicKey,
  int expiresAt,
});

bool isUdpKey(String key) => RegExp(r'^[0-9a-f]{64}$').hasMatch(key);

UdpGateway? udpGatewayFromJson(Object? json) => switch (json) {
  {
    'host': final String host,
    'ports': final List<Object?> ports,
    'publicKey': final String key,
    'expiresAt': final int expiresAt,
  }
      when host.isNotEmpty && isUdpKey(key) && ports.isNotEmpty =>
    (
      host: host,
      ports: [for (final port in ports) ?(port is int ? port : null)],
      publicKey: key,
      expiresAt: expiresAt,
    ),
  _ => null,
};

Future<UdpGateway?>? _asking;

/// The gateway: the kept answer while it is fresh, else the server's (kept
/// for next time), else the kept one even if old; null when the server was
/// never reached. With [refresh] (the gateway did not know the key) the
/// server is asked first.
Future<UdpGateway?> currentUdpGateway({bool refresh = false}) async {
  final prefs = SharedPreferencesAsync();
  final saved = await prefs.getString(_gatewaySettingKey);
  final kept = saved == null ? null : udpGatewayFromJson(_json(saved));
  final now = DateTime.now().millisecondsSinceEpoch ~/ 1000;
  if (!refresh && kept != null && kept.expiresAt > now) return kept;
  return await (_asking ??= _askServer().whenComplete(() => _asking = null)) ??
      kept;
}

/// Replaces the kept key (Developer → Debug). A wrong one is put right by
/// the gateway's wrong-key answer, which makes the app ask again.
Future<void> saveUdpKey(String key) async {
  final gateway = await currentUdpGateway();
  if (gateway == null) return;
  await _keep({
    'host': gateway.host,
    'ports': gateway.ports,
    'publicKey': key,
    'expiresAt': gateway.expiresAt,
  });
}

Future<void> _keep(Map<String, Object?> json) =>
    SharedPreferencesAsync().setString(_gatewaySettingKey, jsonEncode(json));

Object? _json(String text) {
  try {
    return jsonDecode(text);
  } on FormatException {
    return null;
  }
}

Future<UdpGateway?> _askServer() async {
  try {
    final response = await sharedAppHttpClient
        .get(Uri.parse('${await currentServerUrl()}/v1/udp'))
        .timeout(const Duration(seconds: 6));
    final gateway = response.statusCode == 200
        ? udpGatewayFromJson(_json(response.body))
        : null;
    if (gateway != null) await _keep(jsonDecode(response.body));
    return gateway;
  } catch (error) {
    Log.info('vtop.udp', 'no UDP gateway from the server: $error');
    return null;
  }
}

/// The saved switch, for code without a `ref` (read at each call, so
/// background isolates follow it too).
Future<bool> currentVtopUdp() async {
  try {
    return await SharedPreferencesAsync().getBool(vtopUdpSettingKey) ?? false;
  } catch (_) {
    return false;
  }
}

/// The switch, as the Server page shows it.
final vtopUdpProvider = Provider<bool>(
  (ref) =>
      ref.watch(settingsProvider).value?.getBool(vtopUdpSettingKey) ?? false,
);

Future<void> setVtopUdp(WidgetRef ref, bool on) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setBool(vtopUdpSettingKey, on);
  ref.invalidate(settingsProvider);
}

/// [gateway] for the transport, which keeps its device key, sessions and
/// per-network link profiles under the app's support folder.
Future<WireConfig> udpWireConfig(UdpGateway gateway) async => WireConfig(
  host: gateway.host,
  ports: Uint16List.fromList(gateway.ports),
  serverKey: gateway.publicKey,
  stateDir: '${(await getApplicationSupportDirectory()).path}/wire',
);
