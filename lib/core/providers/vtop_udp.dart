/// The experimental UDP mode for "Fetch on the server"
/// ([VtopServerMode.server]): pages come from the UDP gateway
/// (`rust/vtop-gateway`) over the app's own transport (`rust/vtop-wire`),
/// built for slow, lossy networks where HTTPS spends most of its time on
/// handshakes and retransmission timers. Sign-in, OTP and changes to VTOP
/// still go over HTTPS, and so does every page when the gateway cannot be
/// reached.
library;

import 'dart:typed_data';

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/src/api/wire.dart';

const vtopUdpSettingKey = 'settings_vtop_udp';

/// The gateway, built in like `VTOP_SERVER_URL`; a build can change it with
/// `--dart-define` (or `.env.json`). The key is the gateway's public key
/// (`vtop-gateway keygen` prints it as `VTOP_UDP_KEY`).
const udpGatewayHost = String.fromEnvironment(
  'VTOP_UDP_HOST',
  defaultValue: 'vmate-node.aster0.dev',
);
const udpGatewayKey = String.fromEnvironment(
  'VTOP_UDP_KEY',
  defaultValue:
      '12f05307ad08111877036e4312a80ce4327d814ea1fe821ca5ba0ef51e02e04c',
);

/// The gateway's ports, in the order the transport tries them.
final udpGatewayPorts = Uint16List.fromList([3478, 51820, 443]);

/// Whether this build knows a gateway.
bool get udpModeAvailable =>
    udpGatewayHost.isNotEmpty && udpGatewayKey.length == 64;

/// The saved switch, for code without a `ref` (read at each call, so
/// background isolates follow it too).
Future<bool> currentVtopUdp() async {
  if (!udpModeAvailable) return false;
  try {
    return await SharedPreferencesAsync().getBool(vtopUdpSettingKey) ?? false;
  } catch (_) {
    return false;
  }
}

/// The switch, as the Server page shows it.
final vtopUdpProvider = Provider<bool>(
  (ref) =>
      udpModeAvailable &&
      (ref.watch(settingsProvider).value?.getBool(vtopUdpSettingKey) ?? false),
);

Future<void> setVtopUdp(WidgetRef ref, bool on) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setBool(vtopUdpSettingKey, on);
  ref.invalidate(settingsProvider);
}

/// Where the transport keeps its device key, sessions and per-network link
/// profiles.
Future<WireConfig> udpWireConfig() async => WireConfig(
  host: udpGatewayHost,
  ports: udpGatewayPorts,
  serverKey: udpGatewayKey,
  stateDir: '${(await getApplicationSupportDirectory()).path}/wire',
);
