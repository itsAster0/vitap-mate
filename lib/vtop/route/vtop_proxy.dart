/// How the phone reaches VTOP ([VtopServerMode]): itself, through the
/// server's compressing proxy, or with the server signing in and fetching.
/// The server sees the password in both server modes, so it is off unless
/// chosen, and never in on-device mode.
library;

import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/vtop/client_provider.dart';
import 'package:vitapmate/vtop/route/on_device_mode.dart';
import 'package:vitapmate/vtop/route/server_url.dart';
import 'package:vitapmate/app/settings.dart';

enum VtopServerMode {
  /// The phone signs in and fetches, straight to VTOP.
  off,

  /// The phone signs in and fetches through the server's `/vtop/` proxy:
  /// the same requests, but VTOP's pages arrive compressed.
  compress,

  /// The server signs in with the phone's password (asking the phone for
  /// the OTP) and fetches; only the parsed page reaches the phone. What
  /// changes VTOP (outing applies, cancels) and file downloads still go
  /// from the phone, through the proxy. Nothing is kept on the server.
  server,
}

const vtopModeSettingKey = 'settings_vtop_mode';

/// The on/off switch [vtopModeSettingKey] replaced; on reads as
/// [VtopServerMode.compress].
const vtopProxySettingKey = 'settings_vtop_proxy';

/// The setting before it is chosen: compressed through the server, unless
/// the build says `--dart-define=VTOP_PROXY_DEFAULT=false`.
const _proxyByDefault = bool.fromEnvironment(
  'VTOP_PROXY_DEFAULT',
  defaultValue: true,
);

/// The saved mode, from the new key or else the old switch.
VtopServerMode vtopModeFrom({String? saved, bool? oldProxy}) =>
    VtopServerMode.values.asNameMap()[saved] ??
    ((oldProxy ?? _proxyByDefault) ? .compress : .off);

/// The VTOP origin the phone's client uses: the server's proxy of VTOP,
/// or null for VTOP itself.
String? vtopBaseUrlFor({
  required VtopServerMode mode,
  required bool onDevice,
  required String server,
}) => mode != VtopServerMode.off && !onDevice
    ? server.replaceFirst(RegExp(r'/+$'), '')
    : null;

/// The mode in force from the saved settings: [VtopServerMode.off] in
/// on-device mode. For code without a `ref` (it is read at each call, so
/// background isolates follow the setting too).
Future<VtopServerMode> currentVtopMode() async {
  try {
    if (await isOnDeviceMode()) return VtopServerMode.off;
    final prefs = SharedPreferencesAsync();
    return vtopModeFrom(
      saved: await prefs.getString(vtopModeSettingKey),
      oldProxy: await prefs.getBool(vtopProxySettingKey),
    );
  } catch (_) {
    return VtopServerMode.off;
  }
}

/// [vtopBaseUrlFor] from the saved settings, for building a client.
Future<String?> currentVtopBaseUrl() async => vtopBaseUrlFor(
  mode: await currentVtopMode(),
  onDevice: false,
  server: await currentServerUrl(),
);

/// The chosen mode, as the Server page shows it.
final vtopModeProvider = Provider<VtopServerMode>((ref) {
  final prefs = ref.watch(settingsProvider).value;
  return vtopModeFrom(
    saved: prefs?.getString(vtopModeSettingKey),
    oldProxy: prefs?.getBool(vtopProxySettingKey),
  );
});

/// Saves the choice and rebuilds the client, so the next request uses it.
Future<void> setVtopMode(WidgetRef ref, VtopServerMode mode) async {
  final prefs = await ref.read(settingsProvider.future);
  await prefs.setString(vtopModeSettingKey, mode.name);
  await prefs.remove(vtopProxySettingKey);
  ref.invalidate(settingsProvider);
  ref.invalidate(vClientProvider);
}
