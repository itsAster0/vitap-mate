/// The phone's own VTOP requests (sign-in and fetches) through the server:
/// the same requests, but VTOP's uncompressed pages arrive compressed. The
/// server sees them, password included, so it is off unless chosen, and
/// never in on-device mode.
library;

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/di/provider/clinet_provider.dart';
import 'package:vitapmate/core/providers/on_device_mode.dart';
import 'package:vitapmate/core/providers/server_url.dart';
import 'package:vitapmate/core/providers/settings.dart';

const vtopProxySettingKey = 'settings_vtop_proxy';

/// The setting before it is chosen. Off, except on test builds made with
/// `--dart-define=VTOP_PROXY_DEFAULT=true`.
const _proxyByDefault = bool.fromEnvironment('VTOP_PROXY_DEFAULT');

/// The VTOP origin the phone's client uses: the server's proxy of VTOP,
/// or null for VTOP itself.
String? vtopBaseUrlFor({
  required bool proxy,
  required bool onDevice,
  required String server,
}) => proxy && !onDevice ? server.replaceFirst(RegExp(r'/+$'), '') : null;

/// [vtopBaseUrlFor] from the saved settings, for building a client.
Future<String?> currentVtopBaseUrl() async {
  final bool proxy;
  try {
    proxy =
        await SharedPreferencesAsync().getBool(vtopProxySettingKey) ??
        _proxyByDefault;
  } catch (_) {
    return null;
  }
  return vtopBaseUrlFor(
    proxy: proxy,
    onDevice: await isOnDeviceMode(),
    server: await currentServerUrl(),
  );
}

final vtopProxyProvider = Provider<bool>(
  (ref) =>
      ref.watch(settingsProvider).value?.getBool(vtopProxySettingKey) ??
      _proxyByDefault,
);

/// Saves the choice and rebuilds the client, so the next request uses it.
Future<void> setVtopProxy(WidgetRef ref, bool on) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setBool(vtopProxySettingKey, on);
  ref.invalidate(settingsProvider);
  ref.invalidate(vClientProvider);
}
